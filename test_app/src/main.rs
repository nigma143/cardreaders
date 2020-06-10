use cancellation::{CancellationToken, CancellationTokenSource};

use card_less_reader::device::*;
use uno8_nfc_reader::device_builder::Uno8NfcDeviceBuilder;

use cursive::event::{Event, Key};
use cursive::menu::MenuTree;
use cursive::traits::*;
use cursive::views::{Button, DummyView, LinearLayout, TextView};
use cursive::views::{Dialog, EditView, OnEventView, RadioButton, RadioGroup, TextArea};
use cursive::Cursive;
use cursive::{align::HAlign, Printer, Vec2};
use hidapi::HidApi;
use std::sync::{Arc, Mutex};
use std::{
    collections::VecDeque,
    sync::{mpsc, MutexGuard, PoisonError},
    thread,
    time::Duration,
};

struct Session {
    device: Box<dyn CardLessDevice + Send>,
    cb_sink: cursive::CbSink,
}

fn main() {
    // Initialize the cursive logger.
    cursive::logger::init();

    // As usual, create the Cursive root
    let mut siv = cursive::default();

    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);

    connection(&mut siv);

    siv.run();
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
                                o.set_external_display(|x| log::info!("Display: {}", x))
                                    .set_internal_log(|x| log::info!("InternalLog: {}", x))
                                    .set_card_removal(|| log::info!("CardRemoval"))
                                    .finish(),
                            ),
                            cb_sink: x.cb_sink().clone(),
                        })));

                        x.pop_layer();
                        home(x);
                    }
                    Err(e) => x.add_layer(Dialog::info(format!("{}", e))),
                }
            })
            .button("Cancel", |x| x.quit()),
    );
}

fn home(view: &mut Cursive) {
    view.menubar()
        .add_subtree(
            "Commands",
            MenuTree::new()
                .leaf("Get serial number", |x| get_serial_number(x))
                .leaf("External display mode", |x| external_display_mode(x))
                .leaf("Poll emv", |x| poll_emv(x)),
        )
        .add_delimiter()
        .add_leaf("Quit", |s| s.quit());

    view.set_autohide_menu(false);
}

fn get_serial_number(view: &mut Cursive) {
    let session: &mut Arc<Mutex<Session>> = view.user_data().unwrap();
    match Arc::clone(session)
        .lock()
        .unwrap()
        .device
        .get_serial_number()
    {
        Ok(o) => view.add_layer(Dialog::info(format!("{}", o))),
        Err(e) => view.add_layer(Dialog::info(format!("{}", e))),
    };
}

fn external_display_mode(view: &mut Cursive) {
    let mut mode: RadioGroup<ExternalDisplayMode> = RadioGroup::new();

    let mut no_ext_button =
        mode.button(ExternalDisplayMode::NoExternalDisplay, "NoExternalDisplay");
    let mut send_i_button = mode.button(
        ExternalDisplayMode::SendIndexOfPresetMessage,
        "SendIndexOfPresetMessage",
    );
    let mut send_f_button = mode.button(
        ExternalDisplayMode::SendFilteredPresetMessages,
        "SendFilteredPresetMessages",
    );

    let session: &mut Arc<Mutex<Session>> = view.user_data().unwrap();
    match Arc::clone(session)
        .lock()
        .unwrap()
        .device
        .get_external_display_mode()
    {
        Ok(o) => {
            match o {
                ExternalDisplayMode::NoExternalDisplay => no_ext_button.select(),
                ExternalDisplayMode::SendIndexOfPresetMessage => send_i_button.select(),
                ExternalDisplayMode::SendFilteredPresetMessages => send_f_button.select(),
            };
        }
        Err(e) => {
            view.add_layer(Dialog::info(format!("{}", e)));
            return;
        }
    }

    view.add_layer(
        Dialog::new()
            .title("External display mode")
            .content(
                LinearLayout::vertical()
                    .child(no_ext_button)
                    .child(send_i_button)
                    .child(send_f_button),
            )
            .button("Ok", move |s| {
                let mode = mode.selection();

                let session: &mut Arc<Mutex<Session>> = s.user_data().unwrap();
                match Arc::clone(session)
                    .lock()
                    .unwrap()
                    .device
                    .set_external_display_mode(&mode)
                {
                    Ok(o) => {
                        s.pop_layer();
                    }
                    Err(e) => s.add_layer(Dialog::info(format!("{}", e))),
                }
            })
            .button("Cancel", move |s| {
                s.pop_layer();
            }),
    );
}

fn poll_emv(view: &mut Cursive) {
    view.add_layer(
        Dialog::new()
            .title("Poll emv")
            .content(
                LinearLayout::vertical()
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("Payment type: "))
                            .child(
                                EditView::new()
                                    .content("1")
                                    .max_content_width(1)
                                    .with_name("p_type")
                                    .fixed_width(1),
                            ),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("Currency code:"))
                            .child(
                                EditView::new()
                                    .content("643")
                                    .max_content_width(3)
                                    .with_name("currency_code")
                                    .fixed_width(3),
                            ),
                    )
                    .child(
                        LinearLayout::horizontal()
                            .child(TextView::new("Amount       :"))
                            .child(
                                EditView::new()
                                    .content("1234")
                                    .max_content_width(12)
                                    .with_name("amount")
                                    .fixed_width(12),
                            ),
                    ),
            )
            .button("Ok", |x| {
                let p_type = x
                    .call_on_name("p_type", |y: &mut EditView| {
                        u8::from_str_radix(&y.get_content(), 16).unwrap()
                    })
                    .unwrap();
                let currency_code = x
                    .call_on_name("currency_code", |y: &mut EditView| y.get_content())
                    .unwrap()
                    .parse()
                    .unwrap();

                let amount = x
                    .call_on_name("amount", |y: &mut EditView| y.get_content())
                    .unwrap()
                    .parse()
                    .unwrap();

                let session: &mut Arc<Mutex<Session>> = x.user_data().unwrap();

                let cts = CancellationTokenSource::new();
                let ct = cts.token().clone();

                let session_ref = Arc::clone(session);
                thread::spawn(move || {
                    let session = session_ref.lock().unwrap();
                    match session.device.poll_emv(
                        Some(PollEmvPurchase::new(p_type, currency_code, amount)),
                        &ct,
                    ) {
                        Ok(o) => match o {
                            PollEmvResult::Canceled => {}
                            PollEmvResult::Success(tlv) => {
                                session
                                    .cb_sink
                                    .send(Box::new(move |x: &mut cursive::Cursive| {
                                        x.pop_layer();
                                        x.add_layer(Dialog::info(format!("{}", tlv)))
                                    }))
                                    .unwrap();
                            }
                        },
                        Err(e) => {
                            session
                                .cb_sink
                                .send(Box::new(move |x: &mut cursive::Cursive| {
                                    x.pop_layer();
                                    x.add_layer(Dialog::info(format!("{}", e)))
                                }))
                                .unwrap();
                        }
                    };
                });

                x.add_layer(Dialog::text("wait").button("Cancel", move |y| {
                    cts.cancel();
                    y.pop_layer();
                }));
            })
            .button("Cancel", |x| {
                x.pop_layer();
            }),
    );
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
