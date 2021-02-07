//! Configure the MCO (Microcontroller Clock Output) to give a 2MHz signal on PA8 and PA9, sourced
//! from the internal HSI16 16MHz oscillator.

#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{
    pac::{
        self,
        rcc::cfgr::{MCOPRE_A, MCOSEL_A},
    },
    prelude::*,
    rcc::Config,
};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the 16MHz internal clock
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let gpioa = dp.GPIOA.split(&mut rcc);

    // Source MCO from HSI16, configure prescaler to divide by 8 to get 2MHz output.
    rcc.configure_mco(MCOSEL_A::HSI16, MCOPRE_A::DIV8, (gpioa.pa8, gpioa.pa9));

    // Individual pins can also be set by passing them directly:
    // rcc.enable_mco(MCOSEL_A::HSI16, MCOPRE_A::DIV8, gpioa.pa8);

    // Or for larger devices, all 3 MCO pins can be configured:
    // rcc.configure_mco(MCOSEL_A::HSI16, MCOPRE_A::DIV8, (gpioa.pa8, gpioa.pa9, gpiob.pb13));

    // Probe PA8 or PA9 to see generated 2MHz MCO signal.
    loop {}
}
