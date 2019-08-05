//! Interface to the Power control (PWR) peripheral
//!
//! See STM32L0x2 reference manual, chapter 6.


use cortex_m::{
    asm,
    peripheral::SCB,
};

use crate::pac;


/// Entry point to the PWR API
pub struct PWR(pac::PWR);

impl PWR {
    /// Create an instance of the PWR API
    pub fn new(pwr: pac::PWR) -> Self {
        Self(pwr)
    }

    /// Enter Sleep mode
    ///
    /// This method will block until something the microcontroller up again.
    /// Please make sure to configure an interrupt, or this could block forever.
    ///
    /// Please note that this method may change the SCB configuration.
    pub fn enter_sleep_mode(&mut self, scb: &mut SCB) {
        scb.clear_sleepdeep();
        asm::wfi();
    }
}
