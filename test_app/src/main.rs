use cancellation::{CancellationTokenSource, CancellationToken};

use card_less_reader::device::*;
use uno8_nfc_reader::device::ExternalDisplayMode;
use uno8_nfc_reader::device_builder::Uno8NfcDeviceBuilder;

use cursive::event::{Event, Key};
use cursive::traits::*;
use cursive::views::{Dialog, EditView, OnEventView, TextArea};
use cursive::Cursive;
use cursive::views::{DummyView, LinearLayout, TextView};
use cursive::align::HAlign;

fn main() {

    let mut siv = cursive::default();

    // Some description text. We want it to be long, but not _too_ long.
    let text = "This is a very simple example of linear layout. Two views \
                are present, a short title above, and this text. The text \
                has a fixed width, and the title is centered horizontally.";

    // We'll create a dialog with a TextView serving as a title
    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(TextView::new("Title").h_align(HAlign::Center))
                // Use a DummyView as spacer
                .child(DummyView.fixed_height(1))
                // Disabling scrollable means the view cannot shrink.
                .child(TextView::new(text))
                // The other views will share the remaining space.
                .child(TextView::new(text).scrollable())
                .child(TextView::new(text).scrollable())
                .child(TextView::new(text).scrollable()),
        )
        .button("Quit", |s| s.quit())
        .h_align(HAlign::Center),
    );

    siv.run();

    //std::thread::sleep(std::time::Duration::from_millis(5000));

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
    hander.join().unwrap();
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
