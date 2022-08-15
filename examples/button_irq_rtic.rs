#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;
use rtic::app;

#[app(device = stm32l0xx_hal::pac, peripherals = true)]
mod app {
    use stm32l0xx_hal::{
        exti::{Exti, ExtiLine, GpioLine, TriggerEdge},
        gpio::*,
        prelude::*,
        rcc::Config,
        syscfg::SYSCFG,
    };

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
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
        (Shared {}, Local { led }, init::Monotonics())
    }

    #[task(binds = EXTI0_1, local = [ led, state: bool = false ])]
    fn exti0_1(ctx: exti0_1::Context) {
        // Clear the interrupt flag.
        Exti::unpend(GpioLine::from_raw_line(0).unwrap());

        // Change the LED state on each interrupt.
        if *ctx.local.state {
            ctx.local.led.set_low().unwrap();
            *ctx.local.state = false;
        } else {
            ctx.local.led.set_high().unwrap();
            *ctx.local.state = true;
        }
    }
}
