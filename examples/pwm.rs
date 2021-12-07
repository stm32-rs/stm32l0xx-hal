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

    // Initialize TIM2 for PWM
    let pwm = pwm::Timer::new(dp.TIM2, 10_000.Hz(), &mut rcc);

    #[cfg(feature = "stm32l0x1")]
    let mut pwm = pwm.channel2.assign(gpioa.pa1);

    // This is LD2 on ST's B-L072Z-LRWAN1 development board.
    #[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
    let mut pwm = pwm.channel1.assign(gpioa.pa5);

    let max = pwm.get_max_duty();

    pwm.enable();

    loop {
        pwm.set_duty(max);
        delay.delay_ms(500_u16);

        pwm.set_duty(max / 2);
        delay.delay_ms(500_u16);

        pwm.set_duty(max / 4);
        delay.delay_ms(500_u16);

        pwm.set_duty(max / 8);
        delay.delay_ms(500_u16);
    }
}
