#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    let sda = gpioa.pa10.into_open_drain_output();
    let scl = gpioa.pa9.into_open_drain_output();

    let mut i2c = dp.I2C1.i2c(sda, scl, 100.khz(), &mut rcc);

    let mut buffer = [0u8; 2];
    const MAX17048_ADDR: u8 = 0xFF;

    loop {
        i2c.write(MAX17048_ADDR, &mut buffer).unwrap();
    }
}
