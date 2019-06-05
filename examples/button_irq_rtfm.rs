// #![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_halt;
use stm32l0xx_hal as hal;
use hal::{exti::TriggerEdge, gpio::*, pac, prelude::*, rcc::Config};
use embedded_hal::digital::v2::OutputPin;

#[rtfm::app(device = stm32l0xx_hal::pac)]
const APP: () = {
    static mut LED: gpiob::PB6<Output<PushPull>> = ();
    static mut INT: pac::EXTI = ();
    static mut BUTTON: gpiob::PB2<Input<PullUp>> = ();

    #[init]
    fn init() -> init::LateResources {

        // Configure the clock.
        let mut rcc = device.RCC.freeze(Config::hsi16());

        // Acquire the GPIOB peripheral. This also enables the clock for GPIOB in
        // the RCC register.
        let gpiob = device.GPIOB.split(&mut rcc);

        // Configure PB5 as output.
        let led = gpiob.pb6.into_push_pull_output();

        // Configure PB2 as input.
        let button = gpiob.pb2.into_pull_up_input();
        #[cfg(feature = "stm32l0x1")]
        let mut syscfg = device.SYSCFG;
        #[cfg(feature = "stm32l0x2")]
        let mut syscfg = device.SYSCFG_COMP;

        // Configure the external interrupt on the falling edge for the pin 0.
        let exti = device.EXTI;
        exti.listen(
            &mut rcc,
            &mut syscfg,
            button.port,
            button.i,
            TriggerEdge::Falling,
        );

        // Return the initialised resources.
        init::LateResources {
            LED: led,
            INT: exti,
            BUTTON: button,
        }

    }

    #[interrupt(resources = [LED, INT, BUTTON])]
    fn EXTI2_3() {
        static mut STATE: bool = false;

        // Clear the interrupt flag.
        resources.INT.clear_irq(resources.BUTTON.i);
        if *STATE {
           resources.LED.set_low().unwrap();
           *STATE = false;
        } else {
            resources.LED.set_high().unwrap();
           *STATE = true;
        }
        
    }

};
