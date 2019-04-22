// #![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use rtfm::app;
use stm32l0xx_hal::{gpio::*, pac, prelude::*, rcc::Config, timer::Timer};

#[app(device = stm32l0xx_hal::pac)]
const APP: () = {
    static mut LED: gpioa::PA1<Output<PushPull>> = ();
    static mut TIMER: Timer<pac::TIM2> = ();

    #[init]
    fn init() -> init::LateResources {
        // Configure the clock.
        let mut rcc = device.RCC.freeze(Config::hsi16());

        // Acquire the GPIOA peripheral. This also enables the clock for GPIOA
        // in the RCC register.
        let gpioa = device.GPIOA.split(&mut rcc);

        // Configure PA1 as output.
        let led = gpioa.pa1.into_push_pull_output();

        // Configure the timer.
        let mut timer = device.TIM2.timer(1.hz(), &mut rcc);
        timer.listen();

        // Return the initialised resources.
        init::LateResources {
            LED: led,
            TIMER: timer,
        }
    }

    #[interrupt(resources = [LED, TIMER])]
    fn TIM2() {
        static mut STATE: bool = false;

        // Clear the interrupt flag.
        resources.TIMER.clear_irq();

        // Change the LED state on each interrupt.
        if *STATE {
            resources.LED.set_low();
            *STATE = false;
        } else {
            resources.LED.set_high();
            *STATE = true;
        }
    }
};
