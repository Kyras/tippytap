use std::error::Error;
use tippytap::prelude::*;
use std::io::{stdin, Read};

pub fn main() -> Result<(), Box<dyn Error>> {
    let mut device = DeviceBuilder::new(DeviceMode::Tap)
        .packet_info(false)
        .open()?;

    println!("Device: {}", device);
    println!("Device detail: {:?}", device);
    println!("Device name: {}", device.name());
    println!("Press enter key to continue");
    let mut buf = String::new();
    let _ = stdin().read_line(&mut buf);

    let mut buf = [0u8; 65535];
    let read = device.read(&mut buf)
        .expect("failed to read from device");
    println!("{:?}", &buf[..read]);

    Ok(())
}