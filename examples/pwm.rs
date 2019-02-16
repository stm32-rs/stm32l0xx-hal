#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate stm32l0xx_hal as hal;

use core::panic::PanicInfo;
use cortex_m::asm;
use hal::prelude::*;
use hal::rcc::Config;
use hal::stm32;
use rt::entry;

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