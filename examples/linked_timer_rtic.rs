#![no_main]
#![no_std]

extern crate panic_halt;

use core::fmt::Write;

use rtic::app;
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{
    pac,
    rcc::Config,
    serial::{self, Serial},
    time,
    timer::{LinkedTimer, LinkedTimerPair, Timer},
};

const LOGGER_FREQUENCY: u32 = 2;

#[app(device = stm32l0xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        serial: Serial<pac::USART1>,
        timer: Timer<pac::TIM6>,
        linked_tim2_tim3: LinkedTimerPair<pac::TIM2, pac::TIM3>,
        linked_tim21_tim22: LinkedTimerPair<pac::TIM21, pac::TIM22>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let cp: cortex_m::Peripherals = ctx.core;
        let dp: pac::Peripherals = ctx.device;

        // Configure the clock
        let mut rcc = dp.RCC.freeze(Config::hsi16());

        // Get delay provider
        let mut delay = cp.SYST.delay(rcc.clocks);

        // Initialize serial output on PB6 / PB7
        let gpiob = dp.GPIOB.split(&mut rcc);
        let mut serial = Serial::usart1(
            dp.USART1,
            gpiob.pb6.into_floating_input(),
            gpiob.pb7.into_floating_input(),
            serial::Config::default(),
            &mut rcc,
        )
        .unwrap();
        writeln!(serial, "Starting example").ok();

        // Configure the linked timers
        writeln!(serial, "Init TIM2/TIM3...").ok();
        let linked_tim2_tim3 = LinkedTimerPair::tim2_tim3(dp.TIM2, dp.TIM3, &mut rcc);
        delay.delay_ms(1000u16); // 1s offset between timer initialization
        writeln!(serial, "Init TIM21/TIM22...").ok();
        let linked_tim21_tim22 = LinkedTimerPair::tim21_tim22(dp.TIM21, dp.TIM22, &mut rcc);

        // Configure the logging timer
        let mut timer = dp.TIM6.timer(LOGGER_FREQUENCY.hz(), &mut rcc);
        timer.listen();

        init::LateResources {
            serial,
            timer,
            linked_tim2_tim3,
            linked_tim21_tim22,
        }
    }

    #[task(binds = TIM6, resources = [serial, timer, linked_tim2_tim3, linked_tim21_tim22])]
    fn logger(ctx: logger::Context) {
        static mut PREV_TIM2_TIM3: u32 = 0;
        static mut PREV_TIM21_TIM22: u32 = 0;

        // Reset after ~3 seconds
        static mut TIMES_UNTIL_RESET: u32 = 3 * LOGGER_FREQUENCY;

        // Clear the interrupt flag
        ctx.resources.timer.clear_irq();

        // Check reset count
        if *TIMES_UNTIL_RESET > 1 {
            *TIMES_UNTIL_RESET -= 1;
        } else if *TIMES_UNTIL_RESET == 1 {
            writeln!(ctx.resources.serial, "Reset",).ok();
            ctx.resources.linked_tim2_tim3.reset();
            ctx.resources.linked_tim21_tim22.reset();
            *TIMES_UNTIL_RESET -= 1;
        }

        // Print timer counter
        print_timer(
            "TIM2/TIM3   ",
            ctx.resources.linked_tim2_tim3,
            ctx.resources.serial,
            PREV_TIM2_TIM3,
        );
        print_timer(
            "TIM21/TIM22 ",
            ctx.resources.linked_tim21_tim22,
            ctx.resources.serial,
            PREV_TIM21_TIM22,
        );
    }
};

fn print_timer(
    name: &'static str,
    timer: &impl LinkedTimer,
    serial: &mut Serial<pac::USART1>,
    previous: &mut u32,
) {
    // Get the 32 bit counter
    let cnt = timer.get_counter();

    // Difference between current and previous count
    let delta = cnt - *previous;

    // Calculate frequency
    let freq = delta * LOGGER_FREQUENCY / 1000;

    writeln!(
        serial,
        "{} count {:>10} (msb={} lsb={} Δ{} {} kHz)",
        name,
        cnt,
        (cnt & 0xffff0000) >> 16,
        cnt & 0xffff,
        delta,
        freq,
    )
    .ok();

    // Store current count
    *previous = cnt;
}
