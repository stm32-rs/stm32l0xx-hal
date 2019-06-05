  
#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use core::fmt::Write;
use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config, serial};

use nb::block;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    #[cfg(feature = "stm32l0x1")]
    let tx_pin = gpioa.pa9;
    #[cfg(feature = "stm32l0x1")]
    let rx_pin = gpioa.pa10;

    #[cfg(feature = "stm32l0x2")]
    let tx_pin = gpioa.pa2;
    #[cfg(feature = "stm32l0x2")]
    let rx_pin = gpioa.pa3;

    // Configure the serial peripheral.
    let serial = dp
        .USART2
        .usart((tx_pin, rx_pin), serial::Config::default(), &mut rcc)
        .unwrap();

    let (mut tx, mut rx) = serial.split();

    // core::fmt::Write is implemented for tx.
    write!(tx, "Start typing: \r\n").unwrap();

    loop {
        // Echo what is received on the serial link.
        let received = block!(rx.read()).unwrap();
        block!(tx.write(received)).ok();
    }
}