#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::cell::RefCell;
use core::ops::DerefMut;

use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use stm32l0xx_hal::{
    exti::TriggerEdge,
    prelude::*,
    stm32::{self, interrupt, Interrupt, EXTI},
};

static INT: Mutex<RefCell<Option<EXTI>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();

    cp.NVIC.enable(Interrupt::EXTI0_1);
    dp.EXTI.listen(0, TriggerEdge::Falling);

    cortex_m::interrupt::free(move |cs| {
        *INT.borrow(cs).borrow_mut() = Some(dp.EXTI);
    });

    loop {}
}

#[interrupt]
fn EXTI0_1() {
    static mut COUNT: i32 = 0;

    *COUNT += 1;
    hprintln!("CLICK # {}", COUNT).unwrap();

    cortex_m::interrupt::free(|cs| {
        if let &mut Some(ref mut exti) = INT.borrow(cs).borrow_mut().deref_mut() {
            exti.clear_irq(0);
        }
    });
}
