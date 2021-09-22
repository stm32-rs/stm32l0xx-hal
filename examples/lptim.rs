//! Low-Power Timer wakeup.

#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m::{asm, peripheral::NVIC};
use cortex_m_rt::entry;
use nb::block;
use stm32l0xx_hal::{
    exti::{DirectLine, Exti},
    gpio::{Output, Pin, PushPull},
    lptim::{self, ClockSrc, LpTimer},
    pac,
    prelude::*,
    pwr::{self, PWR},
    rcc,
};

#[entry]
fn main() -> ! {
    let cp = pac::CorePeripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut scb = cp.SCB;
    let mut rcc = dp.RCC.freeze(rcc::Config::msi(rcc::MSIRange::Range0));
    let mut exti = Exti::new(dp.EXTI);
    let mut pwr = PWR::new(dp.PWR, &mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    let mut led = gpiob.pb2.into_push_pull_output().downgrade();

    let mut lptim = LpTimer::init_periodic(dp.LPTIM, &mut pwr, &mut rcc, ClockSrc::Lse);

    let exti_line = DirectLine::Lptim1;

    lptim.enable_interrupts(lptim::Interrupts {
        autoreload_match: true,
        ..lptim::Interrupts::default()
    });
    exti.listen_direct(exti_line);

    // Blink twice to signal the start of the program
    blink(&mut led);
    blink(&mut led);

    // 1 seconds of regular run mode
    lptim.start(1.Hz());
    block!(lptim.wait()).unwrap();

    Exti::unpend(exti_line);
    NVIC::unpend(pac::Interrupt::LPTIM1);

    blink(&mut led);

    // 1 seconds of low-power run mode
    pwr.enter_low_power_run_mode(rcc.clocks);
    block!(lptim.wait()).unwrap();
    pwr.exit_low_power_run_mode();
    Exti::unpend(exti_line);
    NVIC::unpend(pac::Interrupt::LPTIM1);

    blink(&mut led);

    // 1 seconds of sleep mode
    exti.wait_for_irq(exti_line, pwr.sleep_mode(&mut scb));
    lptim.wait().unwrap(); // returns immediately; we just got the interrupt
    Exti::unpend(exti_line);
    NVIC::unpend(pac::Interrupt::LPTIM1);

    blink(&mut led);

    // 1 seconds of low-power sleep mode
    exti.wait_for_irq(exti_line, pwr.low_power_sleep_mode(&mut scb, &mut rcc));
    lptim.wait().unwrap(); // returns immediately; we just got the interrupt
    Exti::unpend(exti_line);
    NVIC::unpend(pac::Interrupt::LPTIM1);

    blink(&mut led);

    // 1 seconds of stop mode
    exti.wait_for_irq(
        exti_line,
        pwr.stop_mode(
            &mut scb,
            &mut rcc,
            pwr::StopModeConfig {
                ultra_low_power: true,
            },
        ),
    );
    lptim.wait().unwrap(); // returns immediately; we just got the interrupt

    blink(&mut led);

    // 1 second of standby mode
    NVIC::unpend(pac::Interrupt::LPTIM1);
    exti.wait_for_irq(exti_line, pwr.standby_mode(&mut scb));

    // The microcontroller resets after leaving standby mode. We should never
    // reach this point.
    loop {
        blink(&mut led);
    }
}

fn blink(led: &mut Pin<Output<PushPull>>) {
    led.set_high().unwrap();
    delay();
    led.set_low().unwrap();
    delay();
}

fn delay() {
    // We can't use `Delay`, as that requires a frequency of at least one MHz.
    // Given our clock selection, the following loop should give us a nice delay
    // when compiled in release mode.
    for _ in 0..1_000 {
        asm::nop()
    }
}
