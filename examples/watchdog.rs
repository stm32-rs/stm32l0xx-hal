#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m::asm;
use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let rcc = dp.RCC.freeze(Config::hsi16());

    // Configure a delay to feed the watchdog.
    let mut delay = cp.SYST.delay(rcc.clocks);

    // Configure the independent watchdog.
    let mut watchdog = dp.IWDG.watchdog();

    // Start a watchdog with a 100ms period.
    watchdog.start(10.Hz());

    let mut counter = 50;
    loop {
        // Perform some “work”.
        delay.delay_ms(90_u16);

        // Feed the wathdog on time.
        watchdog.feed();

        counter -= 1;
        if counter == 0 {
            // Block at some point to raise a reset.
            loop {
                asm::nop();
            }
        }
    }
}
