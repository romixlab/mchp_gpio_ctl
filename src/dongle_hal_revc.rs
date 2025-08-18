// Same as on RevA and B:
// PIO0 - PWR_EN_N
// PIO10 - PWR_FAIL_N

// Only from RevC:
// PIO1 - USB_SWITCH_EN
// PIO19 - GPIO header "0"
// PIO20 - GPIO header "1"
// PIO8 - SLG_IO0 (GPIO header "2", not marked)
// PIO3 - SLG_IO1 (GPIO header "3", not marked)

use std::{thread::sleep, time::Duration};

use colored::Colorize;
use nusb::Interface;

use crate::usb4604_ral::{
    modify_reg, read_reg, Gpio0_7Dir, Gpio0_7Input, Gpio0_7Output, Gpio17_20Dir, Gpio17_20Output,
    Gpio41_45Dir, Gpio41_45Output, Gpio8_10Dir, Gpio8_10Output,
};

#[derive(Copy, Clone, Debug)]
pub enum HeaderPin {
    P0,
    P1,
}

#[derive(Copy, Clone, Debug)]
pub enum SlgPin {
    SlgIo0,
    SlgIo1,
}

#[derive(PartialEq, Debug)]
pub enum PinMode {
    Output,
    Input,
}

pub enum PinState {
    High,
    Low,
}

pub fn setup_revc(interface: &Interface) {
    modify_reg::<Gpio0_7Dir, _>(interface, |r| r.set_gpio1_out_en(true));
    slg_io_set_mode(interface, SlgPin::SlgIo0, PinMode::Output);
    slg_io_set_mode(interface, SlgPin::SlgIo1, PinMode::Output);
    gpio_header_set_mode(interface, HeaderPin::P0, PinMode::Output);
    gpio_header_set_mode(interface, HeaderPin::P1, PinMode::Output);

    // for _ in 0..10000 {
    //     gpio_header_set(interface, HeaderPin::P0, PinState::High);
    //     gpio_header_set(interface, HeaderPin::P1, PinState::High);
    //     // usb_switch_set(interface, false);
    //     slg_io_set(interface, SlgPin::SlgIo0, PinState::High);
    //     slg_io_set(interface, SlgPin::SlgIo1, PinState::High);
    //     sleep(Duration::from_millis(100));
    //     gpio_header_set(interface, HeaderPin::P0, PinState::Low);
    //     gpio_header_set(interface, HeaderPin::P1, PinState::Low);
    //     // usb_switch_set(interface, true);
    //     slg_io_set(interface, SlgPin::SlgIo0, PinState::Low);
    //     slg_io_set(interface, SlgPin::SlgIo1, PinState::Low);
    //     sleep(Duration::from_millis(100));
    // }
}

pub fn gpio_header_set_mode(interface: &Interface, pin: HeaderPin, mode: PinMode) {
    let out_en = matches!(mode, PinMode::Output);
    match pin {
        HeaderPin::P0 => {
            modify_reg::<Gpio17_20Dir, _>(interface, |r| r.set_gpio19_out_en(out_en));
        }
        HeaderPin::P1 => {
            modify_reg::<Gpio41_45Dir, _>(interface, |r| r.set_gpio44_out_en(out_en));
        }
    }
}

pub fn gpio_header_get_mode(interface: &Interface, pin: HeaderPin) -> PinMode {
    let is_output = match pin {
        HeaderPin::P0 => read_reg::<Gpio17_20Dir>(interface).gpio19_out_en(),
        HeaderPin::P1 => read_reg::<Gpio41_45Dir>(interface).gpio44_out_en(),
    };
    if is_output {
        PinMode::Output
    } else {
        PinMode::Input
    }
}

pub fn gpio_header_set(interface: &Interface, pin: HeaderPin, state: PinState) {
    if gpio_header_get_mode(interface, pin) != PinMode::Output {
        println!("{}: {pin:?}", "Cannot set pin in input mode".red());
        return;
    }
    let is_high = matches!(state, PinState::High);
    match pin {
        HeaderPin::P0 => {
            modify_reg::<Gpio17_20Output, _>(interface, |r| r.set_gpio19_out(is_high));
        }
        HeaderPin::P1 => {
            modify_reg::<Gpio41_45Output, _>(interface, |r| r.set_gpio44_out(is_high));
        }
    }
}

pub fn slg_io_set_mode(interface: &Interface, pin: SlgPin, mode: PinMode) {
    let out_en = matches!(mode, PinMode::Output);
    match pin {
        SlgPin::SlgIo0 => {
            modify_reg::<Gpio8_10Dir, _>(interface, |r| r.set_gpio8_out_en(out_en));
        }
        SlgPin::SlgIo1 => {
            modify_reg::<Gpio0_7Dir, _>(interface, |r| r.set_gpio3_out_en(out_en));
        }
    }
}

pub fn slg_io_get_mode(interface: &Interface, pin: SlgPin) -> PinMode {
    let is_out_en = match pin {
        SlgPin::SlgIo0 => read_reg::<Gpio8_10Dir>(interface).gpio8_out_en(),
        SlgPin::SlgIo1 => read_reg::<Gpio0_7Dir>(interface).gpio3_out_en(),
    };
    if is_out_en {
        PinMode::Output
    } else {
        PinMode::Input
    }
}

pub fn slg_io_set(interface: &Interface, pin: SlgPin, state: PinState) {
    if slg_io_get_mode(interface, pin) != PinMode::Output {
        println!("{}: {pin:?}", "Cannot set pin in input mode".red());
        return;
    }
    let is_high = matches!(state, PinState::High);
    match pin {
        SlgPin::SlgIo0 => {
            modify_reg::<Gpio8_10Output, _>(interface, |r| r.set_gpio8_out(is_high));
        }
        SlgPin::SlgIo1 => {
            modify_reg::<Gpio0_7Output, _>(interface, |r| r.set_gpio3_out(is_high));
        }
    }
}

pub fn usb_switch_set(interface: &Interface, is_connected: bool) {
    // 0 means the USB switch is connected to a device
    modify_reg::<Gpio0_7Output, _>(interface, |r| r.set_gpio1_out(!is_connected));
}

pub fn usb_switch_is_connected(interface: &Interface) -> bool {
    !read_reg::<Gpio0_7Input>(interface).gpio1_in()
}
