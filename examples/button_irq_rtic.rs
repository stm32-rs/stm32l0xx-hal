// #![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use rtic::app;
use stm32l0xx_hal::{
    exti::{Exti, ExtiLine, GpioLine, TriggerEdge},
    gpio::*,
    prelude::*,
    rcc::Config,
    syscfg::SYSCFG,
};

#[app(device = stm32l0xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        led: Pin<Output<PushPull>>,
        int: Exti,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let device = ctx.device;

        // Configure the clock.
        let mut rcc = device.RCC.freeze(Config::hsi16());

        // Acquire the GPIOB peripheral. This also enables the clock for GPIOB in
        // the RCC register.
        let gpiob = device.GPIOB.split(&mut rcc);

        // Configure PB6 as output.
        let led = gpiob.pb6.into_push_pull_output().downgrade();

        // Configure PB2 as input.
        let button = gpiob.pb2.into_pull_up_input();

        let mut syscfg = SYSCFG::new(device.SYSCFG, &mut rcc);
        let mut exti = Exti::new(device.EXTI);

        // Configure the external interrupt on the falling edge for the pin 0.
        let line = GpioLine::from_raw_line(button.pin_number()).unwrap();
        exti.listen_gpio(&mut syscfg, button.port(), line, TriggerEdge::Falling);

        // Return the initialised resources.
        init::LateResources { led, int: exti }
    }

    #[task(binds = EXTI0_1, resources = [led])]
    fn EXTI0_1(ctx: EXTI0_1::Context) {
        static mut STATE: bool = false;

        // Clear the interrupt flag.
        Exti::unpend(GpioLine::from_raw_line(0).unwrap());

        // Change the LED state on each interrupt.
        if *STATE {
            ctx.resources.led.set_low().unwrap();
            *STATE = false;
        } else {
            ctx.resources.led.set_high().unwrap();
            *STATE = true;
        }
    }
};
