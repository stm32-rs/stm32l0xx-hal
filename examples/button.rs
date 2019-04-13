#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m_rt::entry;
use stm32l0xx_hal::{prelude::*, rcc::Config, stm32};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let rcc = dp.RCC.freeze(Config::hsi16());
    let mut delay = cp.SYST.delay(rcc.clocks);

    let gpioa = dp.GPIOA.split();
    let button = gpioa.pa0.into_pull_up_input();

    let gpiob = dp.GPIOB.split();
    let mut led = gpiob.pb6.into_push_pull_output();

    loop {
        if button.is_high() {
            led.set_high();
            delay.delay(500.ms());
        } else {
            led.set_low();
        }
    }
}
