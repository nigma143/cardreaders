use cancellation::{CancellationToken, CancellationTokenSource};

use card_less_reader::device::*;
use uno8_nfc_reader::device::ExternalDisplayMode;
use uno8_nfc_reader::device_builder::Uno8NfcDeviceBuilder;

use cursive::event::{Event, Key};
use cursive::menu::MenuTree;
use cursive::traits::*;
use cursive::views::{Button, DummyView, LinearLayout, TextView};
use cursive::views::{Dialog, EditView, OnEventView, TextArea};
use cursive::Cursive;
use cursive::{align::HAlign, Printer, Vec2};
use std::sync::{Arc, Mutex};
use std::{collections::VecDeque, sync::mpsc, thread, time::Duration};

struct Session {
    device: Box<dyn CardLessDevice + Send>,
}

fn main() {
    // Initialize the cursive logger.
    cursive::logger::init();

    // As usual, create the Cursive root
    let mut siv = cursive::default();

    let cb_sink = siv.cb_sink().clone();

    siv.add_global_callback('a', cursive::Cursive::toggle_debug_console);

    connection(&mut siv);

    siv.run();

    //std::thread::sleep(std::time::Duration::from_millis(5000));
    /*
    simple_logger::init().unwrap();

    let device = Uno8NfcDeviceBuilder::use_hid(0x1089, 0x0001)
        .unwrap()
        .set_external_display(|x| println!("Display: {}", x))
        .set_internal_log(|x| println!("Log: {}", x))
        .set_card_removal(|| println!("Card removal"))
        .finish();

    device
        .set_external_display_mode(ExternalDisplayMode::SendFilteredPresetMessages)
        .unwrap();

    println!("Serial number: {}", device.get_serial_number().unwrap());

    let cts = CancellationTokenSource::new();
    let ct = cts.token().clone();

    let hander = std::thread::spawn(move || {
        poll_emv(&device, &ct);
    });

    let mut buf = "".to_owned();
    std::io::stdin().read_line(&mut buf).unwrap();

    cts.cancel();
    hander.join().unwrap();*/
}

fn connection(view: &mut Cursive) {
    view.add_layer(
        Dialog::new()
            .title("Connection data")
            .content(
                LinearLayout::vertical()
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("VID(HEX):"))
                            .child(
                                EditView::new()
                                    .content("1089")
                                    .max_content_width(4)
                                    .with_name("vid")
                                    .fixed_width(4),
                            ),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("PID(HEX):"))
                            .child(
                                EditView::new()
                                    .content("0001")
                                    .max_content_width(4)
                                    .with_name("pid")
                                    .fixed_width(4),
                            ),
                    ),
            )
            .button("Ok", |x| {
                let vid = x
                    .call_on_name("vid", |y: &mut EditView| {
                        u16::from_str_radix(&y.get_content(), 16).unwrap()
                    })
                    .unwrap();
                let pid = x
                    .call_on_name("pid", |y: &mut EditView| {
                        u16::from_str_radix(&y.get_content(), 16).unwrap()
                    })
                    .unwrap();

                match Uno8NfcDeviceBuilder::use_hid(vid, pid) {
                    Ok(o) => {
                        x.set_user_data(Arc::new(Mutex::new(Session {
                            device: Box::new(
                                o.set_external_display(|x| log::info!("{}", x)).finish(),
                            ),
                        })));

                        x.pop_layer();
                        home(x);
                    }
                    Err(e) => x.add_layer(Dialog::info(format!("{}", e))),
                }
            }),
    );
}

fn home(view: &mut Cursive) {
    view.menubar()
        .add_subtree(
            "Commands",
            MenuTree::new()
                .leaf("Get serial number", |x| {
                    let session: &mut Arc<Mutex<Session>> = x.user_data().unwrap();
                    match Arc::clone(session)
                        .lock()
                        .unwrap()
                        .device
                        .get_serial_number()
                    {
                        Ok(o) => x.add_layer(Dialog::info(format!("{}", o))),
                        Err(e) => x.add_layer(Dialog::info(format!("{}", e))),
                    }
                })
                .leaf("Poll emv", |x| {
                    let session: &Arc<Mutex<Session>> = x.user_data().unwrap();

                    let cts = CancellationTokenSource::new();
                    let ct = cts.token().clone();

                    let session_ref = Arc::clone(session);

                    let hander = std::thread::spawn(move || {
                        log::info!("get");
                        match session_ref
                            .lock()
                            .unwrap()
                            .device
                            .poll_emv(Some(PollEmvPurchase::new(1, 643, 1000)), &ct)
                        {
                            Ok(r) => match r {
                                PollEmvResult::Canceled => log::info!("cancel"),
                                PollEmvResult::Success(tlv) => log::info!("{}", tlv),
                            },
                            Err(e) => log::info!("{}", e),
                        };
                    });

                    x.add_layer(Dialog::text("wait").button("Cancel", move |y| {
                        cts.cancel();

                        y.pop_layer();
                    }));
                }),
        )
        .add_delimiter()
        .add_leaf("Quit", |s| s.quit());

    view.set_autohide_menu(false);
    /*
    view.add_layer(
        LinearLayout::vertical()
            .child(TextView::new("Serial number:"))
            .child(TextView::new(device.get_serial_number().unwrap()))
            .full_screen(),
    );*/
}

fn poll_emv(device: &impl CardLessDevice, ct: &CancellationToken) {
    let tt = device
        .poll_emv(Some(PollEmvPurchase::new(1, 643, 1000)), &ct)
        .unwrap();

    match tt {
        PollEmvResult::Canceled => println!("canceled"),
        PollEmvResult::Success(x) => println!("{}", x),
    }
}

fn generate_logs(tx: &mpsc::Sender<String>, cb_sink: cursive::CbSink) {
    let mut i = 1;
    loop {
        let line = format!("Interesting log line {}", i);
        i += 1;
        // The send will fail when the other side is dropped.
        // (When the application ends).
        if tx.send(line).is_err() {
            return;
        }
        cb_sink.send(Box::new(Cursive::noop)).unwrap();
        thread::sleep(Duration::from_millis(30));
    }
}

// Let's define a buffer view, that shows the last lines from a stream.
struct BufferView {
    // We'll use a ring buffer
    buffer: VecDeque<String>,
    // Receiving end of the stream
    rx: mpsc::Receiver<String>,
}

impl BufferView {
    // Creates a new view with the given buffer size
    fn new(size: usize, rx: mpsc::Receiver<String>) -> Self {
        let mut buffer = VecDeque::new();
        buffer.resize(size, String::new());
        BufferView { rx, buffer }
    }

    // Reads available data from the stream into the buffer
    fn update(&mut self) {
        // Add each available line to the end of the buffer.
        while let Ok(line) = self.rx.try_recv() {
            self.buffer.push_back(line);
            self.buffer.pop_front();
        }
    }
}

impl View for BufferView {
    fn layout(&mut self, _: Vec2) {
        // Before drawing, we'll want to update the buffer
        self.update();
    }

    fn draw(&self, printer: &Printer) {
        // Print the end of the buffer
        for (i, line) in self.buffer.iter().rev().take(printer.size.y).enumerate() {
            printer.print((0, printer.size.y - 1 - i), line);
        }
    }
}
