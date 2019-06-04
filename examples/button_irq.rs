#![no_main]
#![no_std]

extern crate panic_halt;

use core::cell::RefCell;
use core::ops::DerefMut;

use cortex_m::asm;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use stm32l0xx_hal::{
    exti::TriggerEdge,
    gpio::*,
    pac::{self, Interrupt, EXTI},
    prelude::*,
    rcc::Config,
};

static INT: Mutex<RefCell<Option<EXTI>>> = Mutex::new(RefCell::new(None));
static LED: Mutex<RefCell<Option<gpiob::PB6<Output<PushPull>>>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOB peripheral. This also enables the clock for GPIOB in
    // the RCC register.
    let gpiob = dp.GPIOB.split(&mut rcc);

    // Configure PB6 as output.
    let led = gpiob.pb6.into_push_pull_output();

    // Configure PB2 as input.
    let button = gpiob.pb2.into_pull_up_input();

    #[cfg(feature = "stm32l0x1")]
    let mut syscfg = dp.SYSCFG;
    #[cfg(feature = "stm32l0x2")]
    let mut syscfg = dp.SYSCFG_COMP;

    // Configure the external interrupt on the falling edge for the pin 0.
    let exti = dp.EXTI;
    exti.listen(
        &mut rcc,
        &mut syscfg,
        button.port,
        button.i,
        TriggerEdge::Falling,
    );

    // Store the external interrupt and LED in mutex reffcells to make them
    // available from the interrupt.
    cortex_m::interrupt::free(|cs| {
        *INT.borrow(cs).borrow_mut() = Some(exti);
        *LED.borrow(cs).borrow_mut() = Some(led);
    });

    // Enable the external interrupt in the NVIC.
    let mut nvic = cp.NVIC;
    nvic.enable(Interrupt::EXTI0_1);

    loop {
        asm::wfi();
    }
}

fn EXTI0_1() {
    // Keep the LED state.
    static mut STATE: bool = false;

    cortex_m::interrupt::free(|cs| {
        if let Some(ref mut exti) = INT.borrow(cs).borrow_mut().deref_mut() {
            // Clear the interrupt flag.
            exti.clear_irq(0);

            // Change the LED state on each interrupt.
            if let Some(ref mut led) = LED.borrow(cs).borrow_mut().deref_mut() {
                unsafe {
                    if STATE {
                        led.set_low();
                        STATE = false;
                    } else {
                        led.set_high();
                        STATE = true;
                    }
                }
                
            }
        }
    });
}
