#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::cell::RefCell;
use core::ops::DerefMut;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use stm32l0xx_hal::{
    prelude::*,
    rcc::Config,
    stm32::{self, interrupt, Interrupt},
    timer::Timer,
};

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
    static mut COUNTER: u32 = 0;
    *COUNTER += 1;
    hprintln!("{}", COUNTER).unwrap();

    cortex_m::interrupt::free(|cs| {
        if let Some(ref mut timer) = TIMER.borrow(cs).borrow_mut().deref_mut() {
            timer.clear_irq();
        }
    });
}
