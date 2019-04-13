#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use stm32l0xx_hal::{prelude::*, rcc::Config, stm32};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let rcc = dp.RCC.freeze(Config::hsi16());
    let mut delay = cp.SYST.delay(rcc.clocks);

    //let mut watchdog = dp.WWDG.watchdog(&mut rcc);
    let mut watchdog = dp.IWDG.watchdog();
    watchdog.start(100.ms());

    delay.delay(60.ms());
    //delay.delay(120.ms());

    cortex_m::asm::bkpt();

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
