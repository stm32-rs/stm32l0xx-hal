#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, pwm, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Get the delay provider.
    let mut delay = cp.SYST.delay(rcc.clocks);

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    // Configure TIM2 as PWM on PA1.
    let c2 = gpioa.pa1;
    let mut pwm = pwm::Timer::new(dp.TIM2, c2, 10.khz(), &mut rcc);

    let max = pwm.channels.get_max_duty();

    pwm.channels.enable();

    loop {
        pwm.channels.set_duty(max);
        delay.delay_ms(500_u16);

        pwm.channels.set_duty(max / 2);
        delay.delay_ms(500_u16);

        pwm.channels.set_duty(max / 4);
        delay.delay_ms(500_u16);

        pwm.channels.set_duty(max / 8);
        delay.delay_ms(500_u16);
    }
}
