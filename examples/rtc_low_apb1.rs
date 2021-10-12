//! Real-time clock (RTC) example with low APB1 clock frequency

#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m::asm;
use cortex_m_rt::entry;
use stm32l0xx_hal::{
    pac,
    prelude::*,
    pwr::PWR,
    rcc,
    rtc::{NaiveDate, Rtc, Timelike},
};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // This should put the APB1 clock at 2 times the RTC clock, if I follow the
    // code correctly. Exactly the range that is still acceptable, but requires
    // special handling in the RTC code.
    let mut rcc = dp.RCC.freeze(rcc::Config::msi(rcc::MSIRange::Range0));
    let mut pwr = PWR::new(dp.PWR, &mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    let mut led = gpiob.pb5.into_push_pull_output();

    // Initialize RTC
    let init = NaiveDate::from_ymd(2019, 8, 9).and_hms(13, 37, 42);
    let mut rtc = Rtc::new(dp.RTC, &mut rcc, &mut pwr, Some(init)).unwrap();

    let mut last_second = 0;

    loop {
        let instant = rtc.now();
        if instant.second() != last_second {
            last_second = instant.second();

            // Given the clock settings above, this gives a good blinking going
            // if compiled in release mode.
            led.set_high().unwrap();
            for _ in 0..100 {
                asm::nop()
            }
            led.set_low().unwrap();
        }
    }
}
