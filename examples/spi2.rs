//! An example to show the configuration and usage of SPI1 and SPI2 on `stm32l0x2`, `stm32l0x3` and
//! the subset of `stm32l0x1` series that have two SPI ports
//!
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

    // Initialise the SPI1 peripheral.
    let mut spi = dp
        .SPI1
        .spi((sck, miso, mosi), spi::MODE_0, 100_000.Hz(), &mut rcc);

    let gpiob = dp.GPIOB.split(&mut rcc);

    let mut nss2 = gpiob.pb12.into_push_pull_output();
    let sck2 = gpiob.pb13;
    let miso2 = gpiob.pb14;
    let mosi2 = gpiob.pb15;

    // Initialise the SPI2 peripheral.
    let mut spi2 = dp
        .SPI2
        .spi((sck2, miso2, mosi2), spi::MODE_0, 100_000.Hz(), &mut rcc);

    loop {
        nss.set_low().unwrap();
        spi.write(&[0, 1]).unwrap();
        nss.set_high().unwrap();

        nss2.set_low().unwrap();
        spi2.write(&[0, 1]).unwrap();
        nss2.set_high().unwrap();
    }
}
