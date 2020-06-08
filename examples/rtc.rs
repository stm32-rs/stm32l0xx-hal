//! Real-time clock (RTC) example
//!
//! This example initializes the RTC with a specific time and prints this time
//! to the USART. You should see this time counting up, even if you reset the
//! device, or hold it in reset for a while.
//!
//! If you disconnect the device from power, the RTC should reset to the initial
//! time programmed here after you connect it again. If you press the button,
//! that should rapidly change the seconds while you hold it down.

#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::fmt::Write;

use cortex_m_rt::entry;
use stm32l0xx_hal::{
    pac,
    prelude::*,
    pwr::PWR,
    rcc,
    rtc::{Instant, RTC},
    serial,
};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(rcc::Config::hsi16());
    let mut pwr = PWR::new(dp.PWR, &mut rcc);
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    let button = gpiob.pb2.into_floating_input();

    let serial = dp
        .USART2
        .usart(
            gpioa.pa2,
            gpioa.pa3,
            serial::Config::default().baudrate(115_200.bps()),
            &mut rcc,
        )
        .unwrap();
    let (mut tx, _) = serial.split();

    let instant = Instant::new()
        .set_year(19)
        .set_month(8)
        .set_day(9)
        .set_hour(13)
        .set_minute(36)
        .set_second(0);

    let mut rtc = RTC::new(dp.RTC, &mut rcc, &mut pwr, instant);

    loop {
        let mut instant = rtc.now();

        if button.is_low().unwrap() {
            let second = instant.second() + 1;

            instant = if second < 60 {
                instant.set_second(second)
            } else {
                instant.set_second(0)
            };

            rtc.set(instant);
        }

        write!(
            tx,
            "20{:02}-{:02}-{:02} {:02}:{:02}:{:02}\r\n",
            instant.year(),
            instant.month(),
            instant.day(),
            instant.hour(),
            instant.minute(),
            instant.second(),
        )
        .unwrap();
    }
}
