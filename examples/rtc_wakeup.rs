#![no_main]
#![no_std]


extern crate panic_halt;


use cortex_m_rt::entry;
use stm32l0xx_hal::{
    exti,
    gpio,
    prelude::*,
    pac,
    pwr::{
        self,
        PWR,
    },
    rcc,
    rtc::{
        self,
        Instant,
        RTC,
    },
};


#[entry]
fn main() -> ! {
    let cp = pac::CorePeripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc   = dp.RCC.freeze(rcc::Config::hsi16());
    let mut delay = cp.SYST.delay(rcc.clocks);
    let mut nvic  = cp.NVIC;
    let mut scb   = cp.SCB;
    let mut exti  = dp.EXTI;
    let     gpiob = dp.GPIOB.split(&mut rcc);
    let mut pwr   = PWR::new(dp.PWR, &mut rcc);

    #[cfg(feature = "stm32l0x1")]
    let mut syscfg = dp.SYSCFG;
    #[cfg(feature = "stm32l0x2")]
    let mut syscfg = dp.SYSCFG_COMP;

    let     button = gpiob.pb5.into_floating_input();
    let mut led    = gpiob.pb12.into_push_pull_output();

    // Disable LED
    led.set_high().unwrap();

    let instant = Instant::new()
        .set_year(19)
        .set_month(8)
        .set_day(12)
        .set_hour(12)
        .set_minute(55)
        .set_second(0);

    let mut rtc = RTC::new(
        dp.RTC,
        &mut rcc,
        &mut pwr,
        instant,
    );

    let exti_line = 20; // RTC wakeup timer

    rtc.enable_interrupts(rtc::Interrupts {
        wakeup_timer: true,
        .. rtc::Interrupts::default()
    });
    exti.listen(
        &mut rcc,
        &mut syscfg,
        gpio::Port::PA, // argument ignored; next argument is not a GPIO line
        exti_line,
        exti::TriggerEdge::Rising,
    );

    loop {
        led.set_low().unwrap();
        delay.delay_ms(100u32);
        led.set_high().unwrap();

        while button.is_low().unwrap() {}

        rtc.wakeup_timer().start(1u32);

        exti.wait_for_irq(
            exti_line,
            pwr.stop_mode(
                &mut scb,
                &mut rcc,
                pwr::StopModeConfig {
                    ultra_low_power: true,
                },
            ),
            &mut nvic,
        );
    }
}
