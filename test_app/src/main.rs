use card_less_reader::device::*;
use uno8_nfc_reader::device_builder::Uno8NfcDeviceBuilder;

use cursive::menu::MenuTree;
use cursive::traits::*;
use cursive::views::{Dialog, EditView, RadioGroup};
use cursive::views::{LinearLayout, TextView};
use cursive::Cursive;

use std::sync::{Arc, Mutex};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

struct Session {
    device: Mutex<Box<dyn CardLessDevice>>,
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

                let cb_sink = x.cb_sink().clone();

                match Uno8NfcDeviceBuilder::use_hid(vid, pid) {
                    Ok(o) => {
                        let device = o
                            .set_external_display(move |x| {
                                let message = format!("{}", x);
                                cb_sink
                                    .send(Box::new(move |y: &mut cursive::Cursive| {
                                        y.call_on_name("external_display", |d: &mut TextView| {
                                            d.set_content(message)
                                        });
                                    }))
                                    .unwrap()
                            })
                            .set_internal_log(|x| log::info!("InternalLog: {}", x))
                            .set_card_removal(|| log::info!("CardRemoval"))
                            .finish();

                        x.set_user_data(Arc::new(Session {
                            device: Mutex::new(Box::new(device)),
                        }));

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
    let session = view.user_data::<Arc<Session>>().unwrap().clone();
    let device = session.device.lock().unwrap();

    let mut commands = MenuTree::new();
    commands.add_leaf("Get serial number", |x| get_sn_cmd(x));

    if device.ext_display_supported() {
        commands.add_leaf("External display mode", |x| ext_display_mode_cmd(x));
    }

    commands.add_leaf("Poll emv", |x| poll_emv(x));

    view.menubar()
        .add_subtree("Commands", commands)
        .add_delimiter()
        .add_leaf("Quit", |s| s.quit());

    view.set_autohide_menu(false);
}

fn get_sn_cmd(view: &mut Cursive) {
    let session = view.user_data::<Arc<Session>>().unwrap().clone();
    let device = session.device.lock().unwrap();

    match device.get_sn() {
        Ok(o) => view.add_layer(Dialog::info(format!("{}", o))),
        Err(e) => view.add_layer(Dialog::info(format!("{}", e))),
    };
}

fn ext_display_mode_cmd(view: &mut Cursive) {
    let mut mode: RadioGroup<ExtDisplayMode> = RadioGroup::new();

    let mut no_ext_b = mode.button(ExtDisplayMode::NoDisplay, "NoExternalDisplay");
    let mut send_i_b = mode.button(ExtDisplayMode::Simple, "SendIndexOfPresetMessage");
    let mut send_f_b = mode.button(ExtDisplayMode::Full, "SendFilteredPresetMessages");

    let session = view.user_data::<Arc<Session>>().unwrap().clone();
    let device = session.device.lock().unwrap();
    match device.get_ext_display_mode() {
        Ok(o) => {
            match o {
                ExtDisplayMode::NoDisplay => no_ext_b.select(),
                ExtDisplayMode::Simple => send_i_b.select(),
                ExtDisplayMode::Full => send_f_b.select(),
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
                    .child(no_ext_b)
                    .child(send_i_b)
                    .child(send_f_b),
            )
            .button("Ok", move |x| {
                let mode = mode.selection();

                let session = x.user_data::<Arc<Session>>().unwrap().clone();
                let device = session.device.lock().unwrap();
                match device.set_ext_display_mode(&mode) {
                    Ok(_) => {
                        x.pop_layer();
                    }
                    Err(e) => x.add_layer(Dialog::info(format!("{}", e))),
                }
            })
            .button("Cancel", move |x| {
                x.pop_layer();
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
                let cancel_flag = Arc::new(AtomicBool::new(false));
                let cancel_flag_ref = cancel_flag.clone();

                x.add_layer(
                    Dialog::new()
                        .title("Waiting card")
                        .content(TextView::new("").with_name("external_display"))
                        .button("Cancel", move |y| {
                            cancel_flag.store(true, Ordering::SeqCst);
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

                let session = x.user_data::<Arc<Session>>().unwrap().clone();
                let sb_sink = x.cb_sink().clone();

                thread::spawn(move || {
                    let device = session.device.lock().unwrap();

                    match device.poll_emv(
                        Some(PollEmvPurchase::new(p_type, currency_code, amount)),
                        cancel_flag_ref,
                    ) {
                        Ok(o) => match o {
                            PollEmvResult::Canceled => {}
                            PollEmvResult::Success(tlv) => {
                                sb_sink
                                    .send(Box::new(move |x: &mut cursive::Cursive| {
                                        x.pop_layer();
                                        x.add_layer(Dialog::info(format!("{}", tlv)))
                                    }))
                                    .unwrap();
                            }
                        },
                        Err(e) => {
                            sb_sink
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
