//! Support code for working with the USB peripheral
//!
//! This module is different from the other peripheral APIs, in the sense that
//! it isn't really a peripheral API, just some support code to make working
//! with the USB peripheral possible. This is due to the existing STM32
//! ecosystem support for USB.
//!
//! As USB seems to work mostly the same across all STM32 MCUs, there is a
//! single crates that supports USB for these MCUs:
//! [`stm32-usbd`](https://crates.io/crates/stm32-usbd)
//!
//! Since `stm32-usbd` depends on the HAL libraries for each STM32 family it
//! supports, we can't exactly depend on it to provide everything you need here.
//! Instead, we just provide some code that takes care of the platform-specific
//! bits, which you can call before using `stm32-usbd` for the rest.
//!
//! Please check out the USB examples in the `examples/` directory to see how it
//! fits together.


use crate::{
    pac,
    rcc::Rcc,
    syscfg::SYSCFG,
};


/// Initializes the USB peripheral
///
/// This method takes care of the platform-specific bits of the USB
/// initialization. After calling this method, you need `stm32-usbd` to actually
/// do anything useful with the USB peripheral.
pub fn init(rcc: &mut Rcc, syscfg: &mut SYSCFG, crs: pac::CRS) {
    rcc.enable48_mhz(syscfg, crs);

    // Reset USB peripheral
    rcc.rb.apb1rstr.modify(|_, w| w.usbrst().set_bit());
    rcc.rb.apb1rstr.modify(|_, w| w.usbrst().clear_bit());

    // We don't need to enable the USB peripheral, as stm32-usbd takes care of
    // that.
}
