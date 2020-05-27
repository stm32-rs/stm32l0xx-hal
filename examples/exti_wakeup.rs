#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{
    exti::{Exti, ExtiLine, GpioLine, TriggerEdge},
    pac,
    prelude::*,
    pwr::{self, PWR},
    rcc::Config,
    syscfg::SYSCFG,
};

#[entry]
fn main() -> ! {
    let cp = pac::CorePeripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let gpiob = dp.GPIOB.split(&mut rcc);
    let mut exti = Exti::new(dp.EXTI);
    let mut pwr = PWR::new(dp.PWR, &mut rcc);
    let mut delay = cp.SYST.delay(rcc.clocks);
    let mut scb = cp.SCB;

    // Those are the user button and blue LED on the B-L072Z-LRWAN1 Discovery
    // board.
    let button = gpiob.pb2.into_floating_input();
    let mut led = gpiob.pb6.into_push_pull_output();

    let mut syscfg = SYSCFG::new(dp.SYSCFG, &mut rcc);

    let line = GpioLine::from_raw_line(button.pin_number()).unwrap();

    exti.listen_gpio(&mut syscfg, button.port(), line, TriggerEdge::Falling);

    loop {
        exti.wait_for_irq(
            line,
            pwr.stop_mode(
                &mut scb,
                &mut rcc,
                pwr::StopModeConfig {
                    ultra_low_power: true,
                },
            ),
        );

        led.set_high().unwrap();
        delay.delay_ms(100u32);
        led.set_low().unwrap();
    }
}
