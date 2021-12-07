#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPI0A and GPIOB peripherals. This also enables the clock for
    // GPIOA and GPIOB in the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    // Configure PA0 as input.
    let button = gpioa.pa0.into_pull_up_input();

    // Configure PB6 as output.
    let mut led = gpiob.pb6.into_push_pull_output();

    // Get the delay provider.
    let mut delay = cp.SYST.delay(rcc.clocks);

    loop {
        let wait = match button.is_high() {
            Ok(true) => 300.milliseconds(),
            Ok(false) => 100.milliseconds(),
            _ => unreachable!(),
        };
        delay.delay(wait);
        led.toggle().unwrap();
    }
}
