#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    // Configure the timer as PWM on PA1.
    let mut pwm = dp.TIM2.pwm(gpioa.pa1, 1.khz(), &mut rcc);
    let max_duty = pwm.get_max_duty() / 4095;
    pwm.enable();

    let mut adc = dp.ADC.constrain(&mut rcc);

    // Configure PA0 as analog.
    let mut adc_pin = gpioa.pa0.into_analog();

    loop {
        // Set the PWM duty cycle from the value read on the ADC pin.
        let val: u16 = adc.read(&mut adc_pin).unwrap();
        pwm.set_duty(max_duty * val);
    }
}
