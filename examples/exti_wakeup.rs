#![no_main]
#![no_std]


extern crate panic_halt;


use cortex_m::{
    interrupt,
    peripheral::NVIC,
};
use cortex_m_rt::entry;
use stm32l0xx_hal::{
    prelude::*,
    exti,
    pac::{
        self,
        Interrupt,
    },
    pwr::{
        self,
        PWR,
    },
    rcc::Config,
};


#[entry]
fn main() -> ! {
    let cp = pac::CorePeripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc   = dp.RCC.freeze(Config::hsi16());
    let     gpiob = dp.GPIOB.split(&mut rcc);
    let     exti  = dp.EXTI;
    let mut pwr   = PWR::new(dp.PWR);
    let mut delay = cp.SYST.delay(rcc.clocks);
    let mut nvic  = cp.NVIC;
    let mut scb   = cp.SCB;

    // Those are the user button and blue LED on the B-L072Z-LRWAN1 Discovery
    // board.
    let     button = gpiob.pb2.into_floating_input();
    let mut led    = gpiob.pb6.into_push_pull_output();

    #[cfg(feature = "stm32l0x1")]
    let mut syscfg = dp.SYSCFG;
    #[cfg(feature = "stm32l0x2")]
    let mut syscfg = dp.SYSCFG_COMP;

    exti.listen(
        &mut rcc,
        &mut syscfg,
        button.port,
        button.i,
        exti::TriggerEdge::Falling,
    );

    loop {
        // This construct allows us to wait for the interrupt without having to
        // define an interrupt handler.
        interrupt::free(|_| {
            nvic.enable(Interrupt::EXTI2_3);

            pwr
                .stop_mode(
                    &mut scb,
                    &mut rcc,
                    pwr::StopModeConfig {
                        ultra_low_power: true,
                    },
                )
                .enter();

            exti.clear_irq(button.i);
            NVIC::unpend(Interrupt::EXTI2_3);
            nvic.disable(Interrupt::EXTI2_3);
        });

        led.set_high().unwrap();
        delay.delay_ms(100u32);
        led.set_low().unwrap();
    }
}
