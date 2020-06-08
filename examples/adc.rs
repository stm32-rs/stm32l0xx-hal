#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut adc = dp.ADC.constrain(&mut rcc);
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    let mut a0 = gpioa.pa0.into_analog();

    let mut blue = gpiob.pb6.into_push_pull_output();
    let mut red = gpiob.pb7.into_push_pull_output();

    loop {
        let val: u16 = adc.read(&mut a0).unwrap();

        if val > 2000 {
            blue.set_high().unwrap();
            red.set_low().unwrap();
        } else {
            red.set_high().unwrap();
            blue.set_low().unwrap();
        }
    }
}
