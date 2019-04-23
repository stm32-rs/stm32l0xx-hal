//#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate nb;
extern crate panic_semihosting;
extern crate stm32l0xx_hal as hal;

use cortex_m::asm;
use core::fmt::Write;
use hal::prelude::*;
use hal::rcc::Config;
use hal::serial;
use hal::stm32;
use nb::block;
use rt::entry;

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
        let received = block!(rx.read()).unwrap();
        tx.write(received);
    }
}
