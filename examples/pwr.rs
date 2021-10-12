#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m::{asm, peripheral::NVIC};
use cortex_m_rt::entry;
use stm32l0xx_hal::{
    exti::{ConfigurableLine, Exti, TriggerEdge},
    gpio::{Output, Pin, PushPull},
    pac,
    prelude::*,
    pwr::{self, PWR},
    rcc,
    rtc::{self, Rtc},
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

    // Initialize RTC
    let mut rtc = Rtc::new(dp.RTC, &mut rcc, &mut pwr, None).unwrap();

    // Enable interrupts
    let exti_line = ConfigurableLine::RtcWakeup;
    rtc.enable_interrupts(rtc::Interrupts {
        wakeup_timer: true,
        ..rtc::Interrupts::default()
    });
    exti.listen_configurable(exti_line, TriggerEdge::Rising);

    let mut timer = rtc.wakeup_timer();

    // Blink twice to signal the start of the program
    blink(&mut led);
    blink(&mut led);

    // 5 seconds of regular run mode
    timer.start(5u32);
    while let Err(nb::Error::WouldBlock) = timer.wait() {}
    Exti::unpend(exti_line);
    NVIC::unpend(pac::Interrupt::RTC);

    blink(&mut led);

    // 5 seconds of low-power run mode
    pwr.enter_low_power_run_mode(rcc.clocks);
    while let Err(nb::Error::WouldBlock) = timer.wait() {}
    pwr.exit_low_power_run_mode();
    Exti::unpend(exti_line);
    NVIC::unpend(pac::Interrupt::RTC);

    blink(&mut led);

    // 5 seconds of sleep mode
    exti.wait_for_irq(exti_line, pwr.sleep_mode(&mut scb));
    timer.wait().unwrap(); // returns immediately; we just got the interrupt

    blink(&mut led);

    // 5 seconds of low-power sleep mode
    exti.wait_for_irq(exti_line, pwr.low_power_sleep_mode(&mut scb, &mut rcc));
    timer.wait().unwrap(); // returns immediately; we just got the interrupt

    blink(&mut led);

    // 5 seconds of stop mode
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
    timer.wait().unwrap(); // returns immediately; we just got the interrupt

    blink(&mut led);

    // 5 seconds of standby mode
    cortex_m::peripheral::NVIC::unpend(pac::Interrupt::RTC);
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
