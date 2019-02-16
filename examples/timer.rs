//#![deny(warnings)]
#![no_main]
#![no_std]
#![feature(custom_attribute)]
#[allow(unused)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate stm32l0xx_hal as hal;

use core::panic::PanicInfo;
use core::cell::RefCell;
use core::ops::DerefMut;
use cortex_m::interrupt::Mutex;
use hal::prelude::*;
use hal::rcc::Config;
use hal::stm32::{self, Interrupt};
use hal::timer::Timer;
use cortex_m_rt::entry;

static TIMER: Mutex<RefCell<Option<Timer<stm32::TIM2>>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let mut timer = dp.TIM2.timer(1.hz(), &mut rcc);
    timer.listen();

    cp.NVIC.enable(Interrupt::TIM2);

    cortex_m::interrupt::free(move |cs| {
        *TIMER.borrow(cs).borrow_mut() = Some(timer);
    });

    loop {}
}

#[interrupt]
fn TIM2() {
    //static mut COUNTER: u32 = 0;
    //*COUNTER += 1;

    cortex_m::interrupt::free(|cs| {
        if let Some(ref mut timer) = TIMER.borrow(cs).borrow_mut().deref_mut() {
            timer.clear_irq();
        }
    });
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}