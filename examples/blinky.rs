#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use stm32l0xx_hal::{prelude::*, stm32};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let gpioa = dp.GPIOA.split();
    let mut led = gpioa.pa1.into_push_pull_output();

    loop {
        for _ in 0..1_000 {
            led.set_high();
        }
        for _ in 0..1_000 {
            led.set_low();
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
