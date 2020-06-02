use cancellation::{CancellationTokenSource};

use card_less_reader::device::*;
use uno8_nfc_reader::device::{ExternalDisplayMode};
use uno8_nfc_reader::device_builder::Uno8NfcDeviceBuilder;

fn main() {
    simple_logger::init().unwrap();

    let device = Uno8NfcDeviceBuilder::use_hid(0x1089, 0x0001)
        .unwrap()
        .set_external_display(Box::new(|x| println!("Display: {}", x)))
        .set_internal_log(Box::new(|x| println!("Log: {}", x)))
        .set_card_removal(Box::new(|| println!("Card removal")))
        .finish();

    device
        .set_external_display_mode(ExternalDisplayMode::SendFilteredPresetMessages)
        .unwrap();

    println!("Serial number: {}", device.get_serial_number().unwrap());

    poll_emv(&device);
}

fn poll_emv(device: &impl CardLessDevice) {
    let cts = CancellationTokenSource::new();
    //cts.cancel_after(std::time::Duration::from_millis(1500));

    let tt = device.poll_emv(&cts).unwrap();

    match tt {
        PollEmvResult::Canceled => println!("canceled"),
        PollEmvResult::Success(x) => println!("{}", x),
    }
}
