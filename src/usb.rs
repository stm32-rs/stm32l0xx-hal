//! Interface to the USB peripheral
//!
//! Requires the `stm32-usbd` feature.
//!
//! As USB seems to work mostly the same across all STM32 MCUs, there is a
//! single crate that supports USB for these MCUs:
//! [`stm32-usbd`](https://crates.io/crates/stm32-usbd)
//!
//! This module implements some bits needed for `stm32-usbd` to work and exports
//! `UsbBus` from `stm32-usbd`.
//!
//! Please check out the USB examples in the `examples/` directory to see how it
//! fits together.

use crate::{
    pac,
    rcc::{Enable, Reset, HSI48},
};
use stm32_usbd::UsbPeripheral;

use crate::gpio::gpioa::{PA11, PA12};
use crate::gpio::Analog;
pub use stm32_usbd::UsbBus;

pub struct USB(());

impl USB {
    pub fn new(_: pac::USB, _dm: PA11<Analog>, _dp: PA12<Analog>, _: HSI48) -> Self {
        Self(())
    }
}

unsafe impl Sync for USB {}

unsafe impl UsbPeripheral for USB {
    const REGISTERS: *const () = pac::USB::ptr() as *const ();
    const DP_PULL_UP_FEATURE: bool = true;
    const EP_MEMORY: *const () = 0x4000_6000 as _;
    const EP_MEMORY_SIZE: usize = 1024;
    const EP_MEMORY_ACCESS_2X16: bool = true;

    fn enable() {
        cortex_m::interrupt::free(|_| unsafe {
            // Enable USB peripheral
            pac::USB::enable_unchecked();

            // Reset USB peripheral
            pac::USB::reset_unchecked();
        });
    }

    fn startup_delay() {
        // There is a chip specific startup delay. For STM32L0x2/x3 it's 1µs and this should wait for
        // at least that long.
        cortex_m::asm::delay(32);
    }
}

pub type UsbBusType = UsbBus<USB>;
