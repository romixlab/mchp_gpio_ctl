//! [GPIO Register docs: AN1940](https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ApplicationNotes/ApplicationNotes/00001940C.pdf)
//! [Register docs](https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ApplicationNotes/ApplicationNotes/00001801C.pdf)

use std::time::Duration;

use bitfield_struct::bitfield;
use nusb::{
    transfer::{ControlIn, ControlOut, ControlType, Recipient},
    Interface, MaybeFuture,
};

pub trait SmscReg {
    const ADDR: u16;
    fn from_value(bits: u8) -> Self;
    fn value(&self) -> u8;
}

const CMD_REG_WRITE: u8 = 3;
const CMD_REG_READ: u8 = 4;

pub fn read_reg<R: SmscReg>(interface: &Interface) -> R {
    let read = interface
        .control_in(
            ControlIn {
                control_type: ControlType::Vendor,
                recipient: Recipient::Interface,
                request: CMD_REG_READ,
                value: R::ADDR,
                index: 0,
                length: 1,
            },
            Duration::from_millis(500),
        )
        .wait()
        .unwrap();
    R::from_value(read[0])
}

pub fn write_reg<R: SmscReg>(interface: &Interface, value: R) {
    interface
        .control_out(
            ControlOut {
                control_type: ControlType::Vendor,
                recipient: Recipient::Interface,
                request: CMD_REG_WRITE,
                value: R::ADDR,
                index: 0,
                data: &[value.value()],
            },
            Duration::from_millis(500),
        )
        .wait()
        .unwrap();
}

pub fn modify_reg<R: SmscReg, F: FnMut(&mut R)>(interface: &Interface, mut f: F) {
    let mut value: R = read_reg(interface);
    let old_value = value.value();
    f(&mut value);
    if old_value != value.value() {
        write_reg(interface, value);
    }
}

macro_rules! impl_smsc_reg {
    ($reg_name:ident, $reg_addr:literal) => {
        impl SmscReg for $reg_name {
            const ADDR: u16 = $reg_addr;

            fn from_value(bits: u8) -> Self {
                Self(bits)
            }

            fn value(&self) -> u8 {
                self.0
            }
        }
    };
}

impl_smsc_reg!(Gpio0_7PullDown, 0x082F);
impl_smsc_reg!(Gpio8_10PullDown, 0x082E);
impl_smsc_reg!(Gpio17_20PullDown, 0x082D);
impl_smsc_reg!(Gpio41_45PullDown, 0x092E);

impl_smsc_reg!(Gpio0_7Dir, 0x0833);
impl_smsc_reg!(Gpio8_10Dir, 0x0832);
impl_smsc_reg!(Gpio17_20Dir, 0x0831);
impl_smsc_reg!(Gpio41_45Dir, 0x0932);

impl_smsc_reg!(Gpio0_7Output, 0x0837);
impl_smsc_reg!(Gpio8_10Output, 0x0836);
impl_smsc_reg!(Gpio17_20Output, 0x0835);
impl_smsc_reg!(Gpio41_45Output, 0x0936);

impl_smsc_reg!(Gpio0_7Input, 0x083B);
impl_smsc_reg!(Gpio8_10Input, 0x083A);
impl_smsc_reg!(Gpio17_20Input, 0x0839);
impl_smsc_reg!(Gpio41_45Input, 0x093A);

impl_smsc_reg!(Gpio0_7PullUp, 0x083F);
impl_smsc_reg!(Gpio8_10PullUp, 0x083E);
impl_smsc_reg!(Gpio17_20PullUp, 0x083D);
impl_smsc_reg!(Gpio41_45PullUp, 0x093E);

#[bitfield(u8, order = Msb)]
pub struct Gpio0_7Dir {
    #[bits(1)]
    _gpio7_out_en: bool,
    #[bits(1)]
    _gpio6_out_en: bool,
    #[bits(1)]
    pub gpio5_out_en: bool,
    #[bits(1)]
    _gpio4_out_en: bool,
    #[bits(1)]
    pub gpio3_out_en: bool,

    /// Disable I2C interface before using
    #[bits(1)]
    pub gpio2_out_en: bool,
    #[bits(1)]
    pub gpio1_out_en: bool,
    #[bits(1)]
    pub gpio0_out_en: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio0_7Output {
    #[bits(1)]
    _gpio7_out: bool,
    #[bits(1)]
    _gpio6_out: bool,
    #[bits(1)]
    pub gpio5_out: bool,
    #[bits(1)]
    _gpio4_out: bool,
    #[bits(1)]
    pub gpio3_out: bool,
    #[bits(1)]
    pub gpio2_out: bool,
    #[bits(1)]
    pub gpio1_out: bool,
    #[bits(1)]
    pub gpio0_out: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio0_7Input {
    #[bits(1)]
    _gpio7_in: bool,
    #[bits(1)]
    _gpio6_in: bool,
    #[bits(1)]
    pub gpio5_in: bool,
    #[bits(1)]
    _gpio4_in: bool,
    #[bits(1)]
    pub gpio3_in: bool,
    #[bits(1)]
    pub gpio2_in: bool,
    #[bits(1)]
    pub gpio1_in: bool,
    #[bits(1)]
    pub gpio0_in: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio0_7PullUp {
    #[bits(1)]
    _gpio7_pu: bool,
    #[bits(1)]
    _gpio6_pu: bool,
    #[bits(1)]
    pub gpio5_pu: bool,
    #[bits(1)]
    _gpio4_pu: bool,
    #[bits(1)]
    pub gpio3_pu: bool,
    #[bits(1)]
    pub gpio2_pu: bool,
    #[bits(1)]
    pub gpio1_pu: bool,
    #[bits(1)]
    pub gpio0_pu: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio0_7PullDown {
    #[bits(1)]
    _gpio7_pd: bool,
    #[bits(1)]
    _gpio6_pd: bool,
    #[bits(1)]
    pub gpio5_pd: bool,
    #[bits(1)]
    _gpio4_pd: bool,
    #[bits(1)]
    pub gpio3_pd: bool,
    #[bits(1)]
    pub gpio2_pd: bool,
    #[bits(1)]
    pub gpio1_pd: bool,
    #[bits(1)]
    pub gpio0_pd: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio8_10Dir {
    #[bits(5)]
    _reserved: u8,
    #[bits(1)]
    pub gpio10_out_en: bool,
    #[bits(1)]
    pub gpio9_out_en: bool,
    #[bits(1)]
    pub gpio8_out_en: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio8_10Output {
    #[bits(5)]
    _reserved: u8,
    #[bits(1)]
    pub gpio10_out: bool,
    #[bits(1)]
    pub gpio9_out: bool,
    #[bits(1)]
    pub gpio8_out: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio8_10Input {
    #[bits(5)]
    _reserved: u8,
    #[bits(1)]
    pub gpio10_in: bool,
    #[bits(1)]
    pub gpio9_in: bool,
    #[bits(1)]
    pub gpio8_in: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio8_10PullUp {
    #[bits(5)]
    _reserved: u8,
    #[bits(1)]
    pub gpio10_pu: bool,
    #[bits(1)]
    pub gpio9_pu: bool,
    #[bits(1)]
    pub gpio8_pu: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio8_10PullDown {
    #[bits(5)]
    _reserved: u8,
    #[bits(1)]
    pub gpio10_pd: bool,
    #[bits(1)]
    pub gpio9_pd: bool,
    #[bits(1)]
    pub gpio8_pd: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio17_20Dir {
    #[bits(3)]
    _reserved: u8,

    /// Set hub into OCS Ganged Mode and Disable OCS Input
    /// or Disable Port 4 and Disable OCS Input
    #[bits(1)]
    pub gpio20_out_en: bool,

    /// Set hub into OCS Ganged Mode and Disable OCS Input
    /// or Disable Port 3 and Disable OCS Input
    #[bits(1)]
    pub gpio19_out_en: bool,

    /// Set hub into OCS Ganged Mode and Disable OCS Input
    /// or Disable Port 2 and Disable OCS Input
    #[bits(1)]
    pub gpio18_out_en: bool,

    /// Disable Port 1 and Disable OCS Input
    #[bits(1)]
    pub gpio17_out_en: bool,

    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio17_20Output {
    #[bits(3)]
    _reserved: u8,
    #[bits(1)]
    pub gpio20_out: bool,
    #[bits(1)]
    pub gpio19_out: bool,
    #[bits(1)]
    pub gpio18_out: bool,
    #[bits(1)]
    pub gpio17_out: bool,
    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio17_20Input {
    #[bits(3)]
    _reserved: u8,
    #[bits(1)]
    pub gpio20_in: bool,
    #[bits(1)]
    pub gpio19_in: bool,
    #[bits(1)]
    pub gpio18_in: bool,
    #[bits(1)]
    pub gpio17_in: bool,
    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio17_20PullUp {
    #[bits(3)]
    _reserved: u8,
    #[bits(1)]
    pub gpio20_pu: bool,
    #[bits(1)]
    pub gpio19_pu: bool,
    #[bits(1)]
    pub gpio18_pu: bool,
    #[bits(1)]
    pub gpio17_pu: bool,
    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio17_20PullDown {
    #[bits(3)]
    _reserved: u8,
    #[bits(1)]
    pub gpio20_pd: bool,
    #[bits(1)]
    pub gpio19_pd: bool,
    #[bits(1)]
    pub gpio18_pd: bool,
    #[bits(1)]
    pub gpio17_pd: bool,
    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio41_45Dir {
    #[bits(2)]
    _reserved: u8,

    /// Disable I2C interface
    #[bits(1)]
    pub gpio45_out_en: bool,

    /// Set hub into Port Power Ganged Mode and Disable Port Power Output
    /// or Disable Port 4 and Disable Port Power Output
    #[bits(1)]
    pub gpio44_out_en: bool,

    /// Set hub into Port Power Ganged Mode and Disable Port Power Output
    /// or Disable Port 3 and Disable Port Power Output
    #[bits(1)]
    pub gpio43_out_en: bool,

    /// Set hub into Port Power Ganged Mode and Disable Port Power Output
    /// or Disable Port 2 and Disable Port Power Output
    #[bits(1)]
    pub gpio42_out_en: bool,

    /// Disable Port 1 and Disable Port Power Output
    #[bits(1)]
    pub gpio41_out_en: bool,

    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio41_45Output {
    #[bits(2)]
    _reserved: u8,
    #[bits(1)]
    pub gpio45_out: bool,
    #[bits(1)]
    pub gpio44_out: bool,
    #[bits(1)]
    pub gpio43_out: bool,
    #[bits(1)]
    pub gpio42_out: bool,
    #[bits(1)]
    pub gpio41_out: bool,
    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio41_45Input {
    #[bits(2)]
    _reserved: u8,
    #[bits(1)]
    pub gpio45_in: bool,
    #[bits(1)]
    pub gpio44_in: bool,
    #[bits(1)]
    pub gpio43_in: bool,
    #[bits(1)]
    pub gpio42_in: bool,
    #[bits(1)]
    pub gpio41_in: bool,
    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio41_45PullUp {
    #[bits(2)]
    _reserved: u8,
    #[bits(1)]
    pub gpio45_pu: bool,
    #[bits(1)]
    pub gpio44_pu: bool,
    #[bits(1)]
    pub gpio43_pu: bool,
    #[bits(1)]
    pub gpio42_pu: bool,
    #[bits(1)]
    pub gpio41_pu: bool,
    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Gpio41_45PullDown {
    #[bits(2)]
    _reserved: u8,
    #[bits(1)]
    pub gpio45_pd: bool,
    #[bits(1)]
    pub gpio44_pd: bool,
    #[bits(1)]
    pub gpio43_pd: bool,
    #[bits(1)]
    pub gpio42_pd: bool,
    #[bits(1)]
    pub gpio41_pd: bool,
    #[bits(1)]
    _reserved: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Port3PowerSelect {
    #[bits(1)]
    pub combined_power_select: bool,
    #[bits(1)]
    _reserved: bool,
    #[bits(1)]
    pub disabled: bool,
    #[bits(1)]
    pub permanent: bool,
    #[bits(4)]
    pub prt_sel: u8,
}
impl_smsc_reg!(Port3PowerSelect, 0x3C08);

#[bitfield(u8, order = Msb)]
pub struct HubConfigurationDB0 {
    #[bits(1, access = RO)]
    pub self_bus_pwr: bool,
    #[bits(1, access = RO)]
    pub vsm_disable: bool,
    #[bits(1, access = RO)]
    pub hs_disable: bool,
    #[bits(1, access = RO)]
    pub mtt_enable: bool,
    #[bits(1, access = RO)]
    pub eop_disable: bool,
    #[bits(2, access = RO)]
    pub current_sns: u8,
    #[bits(1, access = RO)]
    pub port_pwr: bool,
}
impl_smsc_reg!(HubConfigurationDB0, 0x3006);
