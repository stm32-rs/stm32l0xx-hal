#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{
    gpio::{Output, Pin, PushPull},
    pac,
    prelude::*,
    rcc::Config,
};

struct Matrix {
    leds: [Pin<Output<PushPull>>; 1],
}

// This example utilizes downgraded pins, to store different pin
// types together in an array. This is useful for building things like
// led matrices, or key scan matrices, etc.
#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    // Configure PA1 as output, using a downgraded Pin which can
    // be put into an array of different pin types.
    let mut matrix = Matrix {
        leds: [gpioa.pa1.into_push_pull_output().downgrade()],
    };

    loop {
        // Loop through all the LEDs in the matrix and set them to high
        for led in matrix.leds.iter_mut() {
            for _ in 0..250_000 {
                led.set_high().unwrap();
            }
        }

        // Loop through all the LEDs and set them to low
        for led in matrix.leds.iter_mut() {
            for _ in 0..250_000 {
                led.set_low().unwrap();
            }
        }
    }
}
