use nusb::Interface;

use crate::usb4604_ral::{
    Gpio0_7Dir, Gpio0_7Output, Gpio8_10Dir, Gpio8_10Input, modify_reg, read_reg,
};

// RevA and RevB board:
// PIO0 - PWR_EN_N
// PIO10 - PWR_FAIL_N

/// Controls the power switch that provides power to a connected device.
pub fn dev_power_ctl(interface: &Interface, pwr_on: bool) {
    modify_reg::<Gpio0_7Dir, _>(interface, |dir| {
        dir.set_gpio0_out_en(true);
    });
    modify_reg::<Gpio0_7Output, _>(interface, |out| {
        out.set_gpio0_out(!pwr_on); // power switch is inverting
    });
}

/// Returns true if power to a connected device is on, default is on in hardware.
pub fn is_dev_power_on(interface: &Interface) -> bool {
    // pin is pulled down with a resistor, even if called after reset (and PIO0 is an input), this should yield correct result
    !read_reg::<Gpio0_7Output>(interface).gpio0_out()
}

/// Returns true if there is a power failure (most likely a short on the output to a device).
pub fn is_dev_pwr_fault(interface: &Interface) -> bool {
    modify_reg::<Gpio8_10Dir, _>(interface, |dir| {
        dir.set_gpio10_out_en(false);
    });
    // fault is inverted
    !read_reg::<Gpio8_10Input>(interface).gpio10_in()
}

#[derive(Debug)]
pub enum PcbRevision {
    RevAorB,
    RevC,
}

pub fn pcb_revision(interface: &Interface) -> PcbRevision {
    let is_revc = read_reg::<Gpio8_10Input>(interface).gpio9_in();
    if is_revc {
        PcbRevision::RevC
    } else {
        PcbRevision::RevAorB
    }
}
