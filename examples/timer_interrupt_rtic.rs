// #![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;
use rtic::app;

#[app(device = stm32l0xx_hal::pac, peripherals = true)]
mod app {
    use stm32l0xx_hal::{gpio::*, pac, prelude::*, rcc::Config, timer::Timer};

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Pin<Output<PushPull>>,
        timer: Timer<pac::TIM2>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let device = ctx.device;

        // Configure the clock.
        let mut rcc = device.RCC.freeze(Config::hsi16());

        // Acquire the GPIOA peripheral. This also enables the clock for GPIOA
        // in the RCC register.
        let gpioa = device.GPIOA.split(&mut rcc);

        // Configure PA1 as output.
        let led = gpioa.pa1.into_push_pull_output().downgrade();

        // Configure the timer.
        let mut timer = device.TIM2.timer(1u32.Hz(), &mut rcc);
        timer.listen();

        // Return the initialised resources.
        (Shared {}, Local { led, timer }, init::Monotonics())
    }

    #[task(binds = TIM2, local = [ led, timer, state: bool = false ])]
    fn tim2(ctx: tim2::Context) {
        // Clear the interrupt flag.
        ctx.local.timer.clear_irq();

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
