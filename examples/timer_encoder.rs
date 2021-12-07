#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{
    encoder::{Mode, Status},
    pac,
    prelude::*,
    rcc::Config,
};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let gpiob = dp.GPIOB.split(&mut rcc);

    // Create an encoder instance that counts between 0 and 64 using inputs on PB13 and PB14.
    let mut encoder = dp
        .TIM21
        .encoder((gpiob.pb13, gpiob.pb14), Mode::Qei, 64, &mut rcc);

    loop {
        #[allow(unused)]
        let Status {
            count,
            did_overflow,
            direction,
        } = encoder.status();

        // Use encoder state here
    }
}
