use std::io::ErrorKind;
use nusb::{Interface, MaybeFuture};
use nusb::transfer::{Control, ControlType, Recipient};
use std::time::Duration;

use clap::{Parser, Subcommand};

// [AN1940](https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ApplicationNotes/ApplicationNotes/00001940C.pdf)
const VENDOR_SMSC: u16 = 0x0424;
const PRODUCT_BRIDGE_DEV: u16 = 0x2530;

const VENDOR_FTDI: u16 = 0x0403;
const PRODUCT_FT234: u16 = 0x6015;

const CMD_REG_WRITE: u8 = 3;
const CMD_REG_READ: u8 = 4;

const PIO0_7_DIR: u16 = 0x0833;
const PIO0_7_OUTPUT: u16 = 0x0837;
const PIO8_15_INPUT: u16 = 0x083a;


#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Serial number of a device to use, can use partial serial number if the result is unique
    #[arg(short, long)]
    serial: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Power on if not already on
    On,
    /// Power off if not already off
    Off,
    /// Print power status
    Status,
    /// List connected devices serials
    List,

    /// Print udev rule to the stdout, run 'mchp_gpio_ctl udev --help' for more information
    ///
    /// Create udev rule:
    /// mchp_gpio_ctl udev | sudo tee /etc/udev/rules.d/70-rm_dongle.rules
    ///
    /// Reload rules and trigger:
    /// sudo udevadm control --reload-rules
    /// sudo udevadm trigger
    #[cfg(target_os = "linux")]
    #[command(verbatim_doc_comment)]
    Udev
}

fn read_reg(interface: &Interface, addr: u16) -> u8 {
    let mut buf = [0u8; 1];
    interface.control_in_blocking(Control {
        control_type: ControlType::Vendor,
        recipient: Recipient::Interface,
        request: CMD_REG_READ,
        value: addr,
        index: 0
    }, &mut buf, Duration::from_millis(500)).unwrap();
    // u32::from_be_bytes(buf)
    buf[0]
}

fn write_reg(interface: &Interface, addr: u16, value: u8) {
    // let buf = value.to_be_bytes();
    let buf = &[value];
    interface.control_out_blocking(Control {
        control_type: ControlType::Vendor,
        recipient: Recipient::Interface,
        request: CMD_REG_WRITE,
        value: addr,
        index: 0
    }, &buf[..], Duration::from_millis(500)).unwrap();
}

fn pwr_ctl(interface: &Interface, turn_on: bool) {
    write_reg(interface, PIO0_7_DIR, 1); // GPIO0 as output
    let mut reg = read_reg(interface, PIO0_7_OUTPUT);
    // println!("{reg:08x}");
    if turn_on {
        reg &= !1; // power switch is inverting
    } else {
        reg |= 1;
    }
    // println!("write {reg:08x}");
    write_reg(interface, PIO0_7_OUTPUT, reg);
}

fn is_pwr_on(interface: &Interface) -> bool {
    let reg = read_reg(&interface, PIO0_7_OUTPUT);
    let is_on = (reg & 1) == 0;
    is_on
}

fn is_pwr_fault(interface: &Interface) -> bool {
    // write_reg(interface, 0x0832, 0);
    let reg = read_reg(&interface, PIO8_15_INPUT);
    let is_fault = (reg & (1 << 2)) == 0; // PIO10_IN
    is_fault
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    let all_devices = nusb::list_devices().wait().unwrap().collect::<Vec<_>>();
    // println!("Devices: {:#?}", all_devices);
    let devices = all_devices.iter().filter(|d| d.vendor_id() == VENDOR_SMSC && d.product_id() == PRODUCT_BRIDGE_DEV).map(|d| {
        let same_hub = d.port_chain();
        let same_hub = &same_hub[..same_hub.len() - 1];
        let ftdi = all_devices.iter().find(|d| d.port_chain().starts_with(&same_hub) && d.vendor_id() == VENDOR_FTDI && d.product_id() == PRODUCT_FT234);
        let serial = ftdi.map(|f| f.serial_number()).flatten().unwrap_or("");
        (d, serial)
    }).collect::<Vec<_>>();
    // println!("{:?}", devices);

    if matches!(cli.command, Commands::List) {
        println!("Connected device list:");
        for (_di, serial) in devices {
            println!("{serial}");
        }
        return;
    }

    let di = if devices.len() == 0 {
        println!("No devices found");
        return;
    } else if devices.len() == 1 {
        match cli.serial {
            Some(serial) => {
                if devices[0].1.contains(&serial) {
                    devices[0].0
                } else {
                    println!("Devices found, but serial provided does not match any of them, device serials:");
                    for (_di, serial) in devices {
                        println!("{serial}");
                    }
                    return;
                }
            }
            None => {
                devices[0].0
            }
        }
    } else {
        match cli.serial {
            Some(serial) => {
                match devices.iter().find(|(_, s)| s.contains(&serial)) {
                    Some((di, _)) => {
                        let total_matches = devices.iter().filter_map(|(_, s)| s.contains(&serial).then_some(())).count();
                        if total_matches == 1 {
                            di
                        } else {
                            println!("Devices found, but serial provided matches more than one device");
                            return;
                        }
                    },
                    None => {
                        println!("Devices found, but serial provided does not match any of them, device serials:");
                        for (_di, serial) in devices {
                            println!("{serial}");
                        }
                        return;
                    }
                }
            }
            None => {
                println!("Several devices connected, please provide serial to select one of them, serials:");
                for (_di, serial) in devices {
                    println!("{serial}");
                }
                return;
            }
        }
    };

    let device = match di.open().wait() {
        Ok(d) => d,
        Err(e) => {
            println!("Failed to open device: {}", e);
            #[cfg(target_os = "linux")]
            if e.kind() == ErrorKind::PermissionDenied {
                println!("You are probably missing an udev rule, run 'mchp_gpio_ctl --help' to see how to install it");
            }
            return;
        }
    };
    let interface = device.claim_interface(0).wait().unwrap();

    let is_pwr_on = is_pwr_on(&interface);
    let is_pwr_fault = is_pwr_fault(&interface);
    if is_pwr_fault {
        println!("Power FAULT detected, probably short on VBUS?");
    }

    match &cli.command {
        Commands::On => {
            if is_pwr_on {
                println!("Power is already ON");
            } else {
                println!("Turning ON...");
                pwr_ctl(&interface, true);
            }
        }
        Commands::Off => {
            if is_pwr_on {
                println!("Turning OFF...");
                pwr_ctl(&interface, false);
            } else {
                println!("Power is already OFF");
            }
        }
        Commands::Status => {
            if is_pwr_on {
                println!("Power is ON");
            } else {
                println!("Power is OFF");
            }
        }
        Commands::List => {}
        #[cfg(target_os = "linux")]
        Commands::Udev => {
            let rule = r#"SUBSYSTEMS=="usb", ATTRS{idVendor}=="0424", ATTRS{idProduct}=="2530", TAG+="uaccess", GROUP="plugdev", MODE="0660""#;
            println!("{rule}");
        }
    }
}
