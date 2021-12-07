#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    // Configure PA1 as output.
    let mut led = gpioa.pa1.into_push_pull_output();

    // Get the delay provider.
    let mut delay = cp.SYST.delay(rcc.clocks);

    loop {
        led.set_high().unwrap();
        delay.delay_ms(500_u16);

        led.set_low().unwrap();
        delay.delay_ms(500_u16);
    }
}
