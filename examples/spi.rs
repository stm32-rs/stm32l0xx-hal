#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config, spi};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut nss = gpioa.pa4.into_push_pull_output();
    let sck = gpioa.pa5;
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7;

    // Initialise the SPI peripheral.
    let mut spi = dp
        .SPI1
        .spi((sck, miso, mosi), spi::MODE_0, 100_000.Hz(), &mut rcc);

    loop {
        nss.set_low().unwrap();
        spi.write(&[0, 1]).unwrap();
        nss.set_high().unwrap();
    }
}
