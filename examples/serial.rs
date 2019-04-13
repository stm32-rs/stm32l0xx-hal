//#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::fmt::Write;
use cortex_m::asm;
use cortex_m_rt::entry;
use nb::block;
use stm32l0xx_hal::{prelude::*, rcc::Config, serial, stm32};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let gpioa = dp.GPIOA.split();
    let tx = gpioa.pa9;
    let rx = gpioa.pa10;

    let serial = dp
        .USART2
        .usart((tx, rx), serial::Config::default(), &mut rcc)
        .unwrap();

    let (mut tx, mut rx) = serial.split();

    loop {
        //tx.write_str("Hello World!\r\n").unwrap();
        let received = block!(rx.read()).unwrap();
        //tx.write_str("Got a byte!\r\n").unwrap();
        tx.write(received);
    }
}
