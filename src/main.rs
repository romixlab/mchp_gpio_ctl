use nusb::MaybeFuture;
use std::thread::sleep;
use std::time::Duration;

use clap::{Parser, Subcommand};
use colored::Colorize;
use mchp_gpio_ctl::dongle_hal_revc::{
    HeaderPin, PinMode, PinState, gpio_header_get, gpio_header_get_mode, gpio_header_set,
    gpio_header_set_mode, slg_io_get, slg_io_set, slg_io_set_mode, usb_switch_configure,
    usb_switch_set,
};
use mchp_gpio_ctl::{
    dongle_hal_revb::{
        PcbRevision, dev_power_ctl, is_dev_power_on, is_dev_pwr_fault, pcb_revision,
    },
    dongle_hal_revc::{SlgPin, usb_switch_is_connected},
};

const VENDOR_SMSC: u16 = 0x0424;
const PRODUCT_BRIDGE_DEV: u16 = 0x2530;
const PRODUCT_USB4604_HUB: u16 = 0x4502;

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
    /// Force SDP for 10 seconds, then go back to USART mode, assuming switch is in USART mode (PCB RevC and up)
    Sdp,
    /// Force SDP mode (Amber LED will blink fast) (PCB RevC and up)
    ForceSdp,
    /// Release to USART mode (Amber LED will not blink, unless switch is in SDP mode) (PCB RevC and up)
    ReleaseSdp,

    /// Disconnect USB data lines from a device via hardware switch (PCB RevC and up)
    Detach,
    /// Connect USB data lines to the device (default) (PCB RevC and up)
    Attach,
    /// Emulate cable detach - disconnect USB data lines, set CC lines to low and disable power to a device (PCB RevC and up)
    FullDetach,
    /// Emulate cable insertion - reconnect USB data lines, set CC lines according to the switch position or force-sdp command, provide power (PCB RevC and up)
    FullAttach,

    /// Configure GPIO header pin (p0 or p1) as Input or Output (e.g., gpio-config p0 output) (PCB RevC and up)
    GpioConfig {
        pin: HeaderPin,
        mode: PinMode,
    },
    /// Set GPIO header pin configured as Output to High or Low (e.g., gpio-set p0 high) (PCB RevC and up)
    GpioSet {
        pin: HeaderPin,
        state: PinState,
    },
    /// Read GPIO header pin state (PCB RevC and up)
    GpioGet {
        pin: HeaderPin,
    },

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
                d.port_chain().starts_with(same_hub)
                    && d.vendor_id() == VENDOR_FTDI
                    && d.product_id() == PRODUCT_FT234
            });
            let serial = ftdi.and_then(|f| f.serial_number()).unwrap_or("");
            let hub = all_devices.iter().find(|d| {
                d.port_chain().starts_with(same_hub) && d.vendor_id() == VENDOR_SMSC && d.product_id() == PRODUCT_USB4604_HUB
            });
            let product_string = hub.and_then(|h| h.product_string()).unwrap_or("");
            (d, serial, product_string)
        })
        .collect::<Vec<_>>();
    // println!("{:?}", devices);

    if matches!(cli.command, Commands::List) {
        println!("Connected device list:");
        for (_di, serial, _product_string) in devices {
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

    let (di, serial, product_string) = if devices.is_empty() {
        println!("No devices found");
        return;
    } else if devices.len() == 1 {
        match cli.serial {
            Some(serial) => {
                if devices[0].1.contains(&serial) {
                    (devices[0].0, devices[0].1, devices[0].2)
                } else {
                    println!(
                        "Devices found, but serial provided does not match any of them, device serials:"
                    );
                    for (_di, serial, _product_string) in devices {
                        println!("{serial}");
                    }
                    return;
                }
            }
            None => (devices[0].0, devices[0].1, devices[0].2)
        }
    } else {
        match cli.serial {
            Some(serial) => match devices.iter().find(|(_, s, _p)| s.contains(&serial)) {
                Some((di, serial, product_string)) => {
                    let total_matches = devices
                        .iter()
                        .filter_map(|(_, s, _p)| s.contains(serial).then_some(()))
                        .count();
                    if total_matches == 1 {
                        (*di, *serial, *product_string)
                    } else {
                        println!("Devices found, but serial provided matches more than one device");
                        return;
                    }
                }
                None => {
                    println!(
                        "Devices found, but serial provided does not match any of them, device serials:"
                    );
                    for (_di, serial, _product_string) in devices {
                        println!("{serial}");
                    }
                    return;
                }
            },
            None => {
                println!(
                    "Several devices connected, please provide serial to select one of them, serials:"
                );
                for (_di, serial, _product_string) in devices {
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
            if e.kind() == nusb::ErrorKind::PermissionDenied || e.os_error() == Some(13) {
                println!(
                    "You are probably missing an udev rule, run 'mchp_gpio_ctl --help' to see how to install it"
                );
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
    // if matches!(pcb_revision, PcbRevision::RevC) {
    // println!("Detected PCB RevC");
    // setup_revc(&interface);
    // }
    let is_relay_variant = product_string.contains("relay");

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
            println!("Dongle serial: {serial}");
            if is_pwr_on {
                println!("Power is ON");
            } else {
                println!("Power is OFF");
            }
            println!("PCB revision: {pcb_revision:?}");
            if is_relay_variant {
                println!("SSR (opto-relay) variant");
            }
            if matches!(pcb_revision, PcbRevision::RevC) {
                println!(
                    "USB switch connected: {}",
                    usb_switch_is_connected(&interface)
                );
                println!(
                    "Is forcing SDP mode: {:?}",
                    slg_io_get(&interface, SlgPin::SlgIo0) == PinState::High
                );
                println!(
                    "Is forcing CC lines down: {:?}",
                    slg_io_get(&interface, SlgPin::SlgIo1) == PinState::Low
                );
                if is_relay_variant {
                    let mode = gpio_header_get_mode(&interface, HeaderPin::P0);
                    if mode == PinMode::Input {
                        println!("{}", "Relay pin p0 is configured as Input, relay won't work".yellow());
                    } else {
                        let state = gpio_header_get(&interface, HeaderPin::P0);
                        if state == PinState::High {
                            println!("Relay state: Short (p0 high)");
                        } else {
                            println!("Relay state: Open (p0 low)");
                        }
                    }
                } else {
                    println!(
                        "Header pin 0 mode: {:?}, state: {:?}",
                        gpio_header_get_mode(&interface, HeaderPin::P0),
                        gpio_header_get(&interface, HeaderPin::P0)
                    );
                }
                println!(
                    "Header pin 1 mode: {:?}, state: {:?}",
                    gpio_header_get_mode(&interface, HeaderPin::P1),
                    gpio_header_get(&interface, HeaderPin::P1)
                );
            }
        }
        Commands::List => {}

        #[cfg(target_os = "linux")]
        Commands::Udev => {}

        Commands::ForceSdp | Commands::ReleaseSdp | Commands::Sdp => {
            if matches!(pcb_revision, PcbRevision::RevAorB) {
                println!("{}", "ForceSDP is not supported on PCB RevA or B".red());
                return;
            }
            slg_io_set_mode(&interface, SlgPin::SlgIo0, PinMode::Output);
            match &cli.command {
                Commands::ForceSdp => {
                    slg_io_set(&interface, SlgPin::SlgIo0, PinState::High);
                }
                Commands::ReleaseSdp => {
                    slg_io_set(&interface, SlgPin::SlgIo0, PinState::Low);
                }
                Commands::Sdp => {
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

        Commands::Attach | Commands::Detach => {
            if matches!(pcb_revision, PcbRevision::RevAorB) {
                println!(
                    "{}",
                    "Attach / Detach is not supported on PCB RevA or B".red()
                );
                return;
            }
            usb_switch_configure(&interface);
            match &cli.command {
                Commands::Attach => {
                    usb_switch_set(&interface, true);
                }
                Commands::Detach => {
                    usb_switch_set(&interface, false);
                }
                _ => {}
            }
        }

        Commands::FullAttach | Commands::FullDetach => {
            if matches!(pcb_revision, PcbRevision::RevAorB) {
                println!(
                    "{}",
                    "Full Attach / Detach is not supported on PCB RevA or B".red()
                );
                return;
            }
            usb_switch_configure(&interface);
            slg_io_set_mode(&interface, SlgPin::SlgIo1, PinMode::Output);
            match &cli.command {
                Commands::FullAttach => {
                    dev_power_ctl(&interface, true);
                    usb_switch_set(&interface, true);
                    slg_io_set(&interface, SlgPin::SlgIo1, PinState::High);
                }
                Commands::FullDetach => {
                    dev_power_ctl(&interface, false);
                    usb_switch_set(&interface, false);
                    slg_io_set(&interface, SlgPin::SlgIo1, PinState::Low);
                }
                _ => {}
            }
        }

        Commands::GpioConfig { .. } | Commands::GpioSet { .. } | Commands::GpioGet { .. } => {
            if matches!(pcb_revision, PcbRevision::RevAorB) {
                println!("{}", "GPIO is not supported on PCB RevA or B".red());
                return;
            }
            match &cli.command {
                Commands::GpioConfig { pin, mode } => {
                    if is_relay_variant && *pin == HeaderPin::P0 && *mode == PinMode::Input {
                        println!("{}", "Configuring relay control pin as input, relay won't work".yellow());
                    }
                    gpio_header_set_mode(&interface, *pin, *mode);
                }
                Commands::GpioSet { pin, state } => {
                    gpio_header_set(&interface, *pin, *state);
                }
                Commands::GpioGet { pin } => {
                    let state = gpio_header_get(&interface, *pin);
                    println!("{pin:?} = {state:?}");
                }
                _ => {}
            }
        }
    }
}
