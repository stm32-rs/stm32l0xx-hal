// #![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use rtic::app;
use stm32l0xx_hal::{gpio::*, pac, prelude::*, rcc::Config, timer::Timer};

#[app(device = stm32l0xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        led: Pin<Output<PushPull>>,
        timer: Timer<pac::TIM2>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let device = ctx.device;

        // Configure the clock.
        let mut rcc = device.RCC.freeze(Config::hsi16());

        // Acquire the GPIOA peripheral. This also enables the clock for GPIOA
        // in the RCC register.
        let gpioa = device.GPIOA.split(&mut rcc);

        // Configure PA1 as output.
        let led = gpioa.pa1.into_push_pull_output().downgrade();

        // Configure the timer.
        let mut timer = device.TIM2.timer(1.Hz(), &mut rcc);
        timer.listen();

        // Return the initialised resources.
        init::LateResources { led, timer }
    }

    #[task(binds = TIM2, resources = [led, timer])]
    fn TIM2(ctx: TIM2::Context) {
        static mut STATE: bool = false;

        // Clear the interrupt flag.
        ctx.resources.timer.clear_irq();

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
