#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

//use core::fmt::Write;
use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, pwr::PWR, rcc::Config, serial};

use nb::block;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let pwr = PWR::new(dp.PWR, &mut rcc);
    let lse = rcc.enable_lse(&pwr);

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    // Choose TX / RX pins
    let tx_pin = gpioa.pa2;
    let rx_pin = gpioa.pa3;

    // Configure the serial peripheral.
    let mut serial = dp
        .LPUART1
        .usart(tx_pin, rx_pin, serial::Config::default(), &mut rcc)
        .unwrap();
    serial.use_lse(&mut rcc, &lse);
    let (mut tx, mut rx) = serial.split();

    // core::fmt::Write is implemented for tx.
    //writeln!(tx, "Hello, world!").unwrap();

    loop {
        // Echo what is received on the serial link.
        let received = block!(rx.read()).unwrap();
        block!(tx.write(received)).ok();
    }
}
