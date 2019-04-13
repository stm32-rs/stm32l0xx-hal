#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m_rt::entry;
use stm32l0xx_hal::{prelude::*, rcc::Config, spi, stm32};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let gpioa = dp.GPIOA.split();

    let sck = gpioa.pa5;
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7;

    let mut spi = spi::Spi::spi1(
        dp.SPI1,
        (sck, miso, mosi),
        spi::MODE_0,
        100_000.hz(),
        &mut rcc,
    );

    loop {
        spi.write(&[0, 1]).unwrap();
    }
}
