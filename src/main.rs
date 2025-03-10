use nusb::Device;
use nusb::transfer::{Control, ControlType, Recipient};
use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    PwrOn,
    PwrOff,
    PwrState
}

fn read_reg(device: &Device, addr: u16) -> u32 {
    let mut buf = [0u8; 4];
    device.control_in_blocking(Control {
        control_type: ControlType::Vendor,
        recipient: Recipient::Interface,
        request: 4,
        value: addr,
        index: 0
    }, &mut buf, Duration::from_millis(500)).unwrap();
    u32::from_be_bytes(buf)
}

fn write_reg(device: &Device, addr: u16, value: u32) {
    let buf = value.to_be_bytes();
    device.control_out_blocking(Control {
        control_type: ControlType::Vendor,
        recipient: Recipient::Interface,
        request: 3,
        value: addr,
        index: 0
    }, &buf, Duration::from_millis(500)).unwrap();
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    let di = nusb::list_devices()
        .unwrap()
        .find(|d| d.vendor_id() == 0x0424 && d.product_id() == 0x2530)
        .expect("device should be connected");
    println!("Device info: {di:?}");

    let device = di.open().unwrap();

//    let interface = device.claim_interface(1).unwrap();

    match &cli.command {
        Commands::PwrOn => {
            write_reg(&device, 0x0830, 1); // GPIO0 as output
            let mut reg = read_reg(&device, 0x0834);
            println!("{reg:08x}");
            reg |= 1;
            println!("write {reg:08x}");
            write_reg(&device, 0x0834, reg);
        }
        Commands::PwrOff => {
            write_reg(&device, 0x0830, 1); // GPIO0 as output
            let mut reg = read_reg(&device, 0x0834);
            println!("{reg:08x}");
            reg &= !1;
            println!("write {reg:08x}");
            write_reg(&device, 0x0834, reg);
        }
        Commands::PwrState => {

        }
    }

}
