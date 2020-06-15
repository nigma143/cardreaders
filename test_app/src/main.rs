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
use mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{
    collections::VecDeque,
    sync::{mpsc, MutexGuard, PoisonError},
    thread,
    time::Duration,
};

struct Session {
    device: Box<(dyn CardLessDevice + Send)>,
    display_source: Receiver<String>
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
                        let (display_tx, display_rx) = mpsc::channel();

                        let device = o
                            .set_external_display(move |x| display_tx.send(x.to_owned()).unwrap())
                            .set_internal_log(|x| log::info!("InternalLog: {}", x))
                            .set_card_removal(|| log::info!("CardRemoval"))
                            .finish();

                        x.set_user_data(Arc::new(Mutex::new(Session {
                            device: Box::new(device),
                            display_source: display_rx
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
                .leaf("Get serial number", |x| get_sn_cmd(x))
                .leaf("External display mode", |x| ext_display_mode_cmd(x))
                .leaf("Poll emv", |x| poll_emv(x)),
        )
        .add_delimiter()
        .add_leaf("Quit", |s| s.quit());

    view.set_autohide_menu(false);
}

fn get_sn_cmd(view: &mut Cursive) {
    let session: &mut Arc<Mutex<Session>> = view.user_data().unwrap();
    match Arc::clone(session).lock().unwrap().device.get_sn() {
        Ok(o) => view.add_layer(Dialog::info(format!("{}", o))),
        Err(e) => view.add_layer(Dialog::info(format!("{}", e))),
    };
}

fn ext_display_mode_cmd(view: &mut Cursive) {
    let mut mode: RadioGroup<ExtDisplayMode> = RadioGroup::new();

    let mut no_ext_button = mode.button(ExtDisplayMode::NoDisplay, "NoExternalDisplay");
    let mut send_i_button = mode.button(ExtDisplayMode::Simple, "SendIndexOfPresetMessage");
    let mut send_f_button = mode.button(ExtDisplayMode::Full, "SendFilteredPresetMessages");

    let session: &mut Arc<Mutex<Session>> = view.user_data().unwrap();
    match Arc::clone(session)
        .lock()
        .unwrap()
        .device
        .get_ext_display_mode()
    {
        Ok(o) => {
            match o {
                ExtDisplayMode::NoDisplay => no_ext_button.select(),
                ExtDisplayMode::Simple => send_i_button.select(),
                ExtDisplayMode::Full => send_f_button.select(),
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
                    .set_ext_display_mode(&mode)
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
                let cts = CancellationTokenSource::new();
                let ct = cts.token().clone();

                x.add_layer(
                    Dialog::text("wait")
                        .content(TextView::new("").with_name("display"))
                        .button("Cancel", move |y| {
                            cts.cancel();
                            y.pop_layer();
                        }),
                );

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

                let session_ref = Arc::clone(session);
                let sb_sink = x.cb_sink().clone();
                let sb_sink2 = x.cb_sink().clone();

                thread::spawn(move || {
                    let mut session = session_ref.lock().unwrap();

                    session.device.set_ext_display(Box::new(move |x| {
                        let message = format!("{}", x);
                        sb_sink
                            .send(Box::new(move |y: &mut cursive::Cursive| {
                                y.call_on_name("display", |d: &mut TextView| {
                                    d.set_content(message)
                                });
                            }))
                            .unwrap()
                    }));

                    match session.device.poll_emv(
                        Some(PollEmvPurchase::new(p_type, currency_code, amount)),
                        &ct,
                    ) {
                        Ok(o) => match o {
                            PollEmvResult::Canceled => {}
                            PollEmvResult::Success(tlv) => {
                                sb_sink2
                                    .send(Box::new(move |x: &mut cursive::Cursive| {
                                        x.pop_layer();
                                        x.add_layer(Dialog::info(format!("{}", tlv)))
                                    }))
                                    .unwrap();
                            }
                        },
                        Err(e) => {
                            sb_sink2
                                .send(Box::new(move |x: &mut cursive::Cursive| {
                                    x.pop_layer();
                                    x.add_layer(Dialog::info(format!("{}", e)))
                                }))
                                .unwrap();
                        }
                    };
                });
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
