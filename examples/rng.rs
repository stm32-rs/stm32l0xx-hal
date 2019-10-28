#![no_main]
#![no_std]
//#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rng, rcc::Config, syscfg::SYSCFG};

use core::fmt::Write;
use stm32l0xx_hal::serial;
use stm32l0xx_hal::rng::Rng;
#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut syscfg = SYSCFG::new(dp.SYSCFG_COMP, &mut rcc);

    // constructor initializes 48 MHz clock that RNG requires
    let mut rng = Rng::new(dp.RNG, &mut rcc, &mut syscfg, dp.CRS);

    loop {
        // enable starts the ADC conversions that generate the random number
        rng.enable();
        // wait until the flag flips; interrupt driven is possible but no implemented
        rng.wait();
        // reading the result clears the ready flag
        let val = rng.take_result();
        // can save some power by disabling until next random number needed
        rng.disable();
    }
}