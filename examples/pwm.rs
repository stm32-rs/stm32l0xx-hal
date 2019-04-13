#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

use core::panic::PanicInfo;
use cortex_m::asm;
use cortex_m_rt::entry;
use stm32l0xx_hal::{prelude::*, rcc::Config, stm32};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let gpioa = dp.GPIOA.split();

    let c1 = gpioa.pa0;
    let mut pwm = dp.TIM2.pwm(c1, 10.khz(), &mut rcc);

    let max = pwm.get_max_duty();

    pwm.enable();

    pwm.set_duty(max);
    asm::bkpt();

    pwm.set_duty(max / 2);
    asm::bkpt();

    pwm.set_duty(max / 4);
    asm::bkpt();

    pwm.set_duty(max / 8);
    asm::bkpt();

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
