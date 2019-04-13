#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use stm32l0xx_hal::{prelude::*, rcc::Config, stm32};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let gpioa = dp.GPIOA.split();

    let mut adc_pin = gpioa.pa1.into_analog();
    let mut adc = dp.ADC.adc(&mut rcc);

    let mut pwm = dp.TIM2.pwm(gpioa.pa0, 1.khz(), &mut rcc);
    let max_duty = pwm.get_max_duty() / 4095;
    pwm.enable();

    loop {
        let val: u16 = adc.read(&mut adc_pin).unwrap();
        pwm.set_duty(max_duty * val);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
