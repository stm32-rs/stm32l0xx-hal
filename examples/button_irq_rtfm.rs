// #![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use rtfm::app;
use stm32l0xx_hal::{
    exti::{TriggerEdge, Exti, GpioLine, ExtiLine},
    gpio::*,
    prelude::*,
    rcc::Config,
    syscfg::SYSCFG,
};

#[app(device = stm32l0xx_hal::pac)]
const APP: () = {
    static mut LED: gpiob::PB6<Output<PushPull>> = ();
    static mut INT: Exti = ();

    #[init]
    fn init() -> init::LateResources {
        // Configure the clock.
        let mut rcc = device.RCC.freeze(Config::hsi16());

        // Acquire the GPIOB peripheral. This also enables the clock for GPIOB in
        // the RCC register.
        let gpiob = device.GPIOB.split(&mut rcc);

        // Configure PB6 as output.
        let led = gpiob.pb6.into_push_pull_output();

        // Configure PB2 as input.
        let button = gpiob.pb2.into_pull_up_input();

        let mut syscfg = SYSCFG::new(device.SYSCFG, &mut rcc);
        let mut exti = Exti::new(device.EXTI);

        // Configure the external interrupt on the falling edge for the pin 0.
        let line = GpioLine::from_raw_line(button.pin_number()).unwrap();
        exti.listen_gpio(&mut syscfg, button.port(), line, TriggerEdge::Falling);

        // Return the initialised resources.
        init::LateResources {
            LED: led,
            INT: exti,
        }
    }

    #[interrupt(resources = [LED, INT])]
    fn EXTI0_1() {
        static mut STATE: bool = false;

        // Clear the interrupt flag.
        Exti::unpend(GpioLine::from_raw_line(0).unwrap());

        // Change the LED state on each interrupt.
        if *STATE {
            resources.LED.set_low().unwrap();
            *STATE = false;
        } else {
            resources.LED.set_high().unwrap();
            *STATE = true;
        }
    }
};
