use std::thread::sleep;
use std::time::Duration;
use nusb::MaybeFuture;

use clap::{Parser, Subcommand};
use colored::Colorize;
use mchp_gpio_ctl::dongle_hal_revc::{slg_io_set, PinState};
use mchp_gpio_ctl::{
    dongle_hal_revb::{
        dev_power_ctl, is_dev_power_on, is_dev_pwr_fault, pcb_revision, PcbRevision,
    },
    dongle_hal_revc::{setup_revc, slg_io_get_mode, usb_switch_is_connected, SlgPin},
};

const VENDOR_SMSC: u16 = 0x0424;
const PRODUCT_BRIDGE_DEV: u16 = 0x2530;

const VENDOR_FTDI: u16 = 0x0403;
const PRODUCT_FT234: u16 = 0x6015;

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
    /// Print dongle information (power status, IO config)
    Status,
    /// List connected devices serials
    List,

    // Only on RevC
    /// Force SDP for 10 seconds, then go back to USART mode
    SDP,
    /// Force SDP mode (Amber LED will blink fast)
    ForceSDP,
    /// Release to USART mode (Amber LED will not blink, unless switch is in SDP mode)
    ReleaseSDP,

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
    Udev,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    let all_devices = nusb::list_devices().wait().unwrap().collect::<Vec<_>>();
    // println!("Devices: {:#?}", all_devices);
    let devices = all_devices
        .iter()
        .filter(|d| d.vendor_id() == VENDOR_SMSC && d.product_id() == PRODUCT_BRIDGE_DEV)
        .map(|d| {
            let same_hub = d.port_chain();
            let same_hub = &same_hub[..same_hub.len() - 1];
            let ftdi = all_devices.iter().find(|d| {
                d.port_chain().starts_with(&same_hub)
                    && d.vendor_id() == VENDOR_FTDI
                    && d.product_id() == PRODUCT_FT234
            });
            let serial = ftdi.map(|f| f.serial_number()).flatten().unwrap_or("");
            (d, serial)
        })
        .collect::<Vec<_>>();
    // println!("{:?}", devices);

    if matches!(cli.command, Commands::List) {
        println!("Connected device list:");
        for (_di, serial) in devices {
            println!("{serial}");
        }
        return;
    }
    #[cfg(target_os = "linux")]
    if matches!(cli.command, Commands::Udev) {
        println!(
            r#"SUBSYSTEMS=="usb", ATTRS{{idVendor}}=="{:04x}", ATTRS{{idProduct}}=="{:04x}", TAG+="uaccess", GROUP="plugdev", MODE="0660""#,
            VENDOR_SMSC, PRODUCT_BRIDGE_DEV
        );
        println!(
            r#"SUBSYSTEMS=="usb", ATTRS{{idVendor}}=="{:04x}", ATTRS{{idProduct}}=="{:04x}", TAG+="uaccess", GROUP="plugdev", MODE="0660""#,
            VENDOR_FTDI, PRODUCT_FT234
        );
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
            None => devices[0].0,
        }
    } else {
        match cli.serial {
            Some(serial) => match devices.iter().find(|(_, s)| s.contains(&serial)) {
                Some((di, _)) => {
                    let total_matches = devices
                        .iter()
                        .filter_map(|(_, s)| s.contains(&serial).then_some(()))
                        .count();
                    if total_matches == 1 {
                        di
                    } else {
                        println!("Devices found, but serial provided matches more than one device");
                        return;
                    }
                }
                None => {
                    println!("Devices found, but serial provided does not match any of them, device serials:");
                    for (_di, serial) in devices {
                        println!("{serial}");
                    }
                    return;
                }
            },
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
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                println!("You are probably missing an udev rule, run 'mchp_gpio_ctl --help' to see how to install it");
            }
            return;
        }
    };
    let interface = device.claim_interface(0).wait().unwrap();

    let is_pwr_on = is_dev_power_on(&interface);
    let is_pwr_fault = is_dev_pwr_fault(&interface);
    if is_pwr_fault {
        println!("{}", "Power FAULT detected, probably short on VBUS?".red());
    }
    let pcb_revision = pcb_revision(&interface);
    if matches!(pcb_revision, PcbRevision::RevC) {
        println!("Detected PCB RevC");
        setup_revc(&interface);
    }

    match &cli.command {
        Commands::On => {
            if is_pwr_on {
                println!("Power is already ON");
            } else {
                println!("Turning ON...");
                dev_power_ctl(&interface, true);
            }
        }
        Commands::Off => {
            if is_pwr_on {
                println!("Turning OFF...");
                dev_power_ctl(&interface, false);
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
            println!("PCB revision: {pcb_revision:?}");
            if matches!(pcb_revision, PcbRevision::RevC) {
                // TODO: GPIO config

                println!(
                    "USB switch connected: {}",
                    usb_switch_is_connected(&interface)
                );
                println!(
                    "SLG0 pin mode: {:?}",
                    slg_io_get_mode(&interface, SlgPin::SlgIo0)
                );
                println!(
                    "SLG1 pin mode: {:?}",
                    slg_io_get_mode(&interface, SlgPin::SlgIo1)
                );
            }
        }
        Commands::List => {}

        #[cfg(target_os = "linux")]
        Commands::Udev => {}

        Commands::ForceSDP | Commands::ReleaseSDP | Commands::SDP => {
            if matches!(pcb_revision, PcbRevision::RevAorB) {
                println!("{}", "ForceSDP is not supported on PCB RevA or B".red());
                return;
            }
            match &cli.command {
                Commands::ForceSDP => {
                    slg_io_set(&interface, SlgPin::SlgIo0, PinState::High);
                }
                Commands::ReleaseSDP => {
                    slg_io_set(&interface, SlgPin::SlgIo0, PinState::Low);
                }
                Commands::SDP => {
                    slg_io_set(&interface, SlgPin::SlgIo0, PinState::High);
                    for i in (1..=10).rev() {
                        println!("{i}");
                        sleep(Duration::from_secs(1));
                    }
                    slg_io_set(&interface, SlgPin::SlgIo0, PinState::Low);
                }
                _ => {}
            }
        }
    }
}
