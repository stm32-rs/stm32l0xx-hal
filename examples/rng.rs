#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config, syscfg::SYSCFG};

use stm32l0xx_hal::rng::Rng;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut syscfg = SYSCFG::new(dp.SYSCFG, &mut rcc);

    // constructor initializes 48 MHz clock that RNG requires
    // Initialize 48 MHz clock and RNG
    let hsi48 = rcc.enable_hsi48(&mut syscfg, dp.CRS);
    let mut rng = Rng::new(dp.RNG, &mut rcc, hsi48);

    loop {
        // enable starts the ADC conversions that generate the random number
        rng.enable();
        // wait until the flag flips; interrupt driven is possible but no implemented
        rng.wait();
        // reading the result clears the ready flag
        let _ = rng.take_result();
        // can save some power by disabling until next random number needed
        rng.disable();
    }
}
