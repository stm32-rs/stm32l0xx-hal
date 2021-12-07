#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m_rt::entry;
use stm32l0xx_hal::{
    flash::{flash_size_in_kb, FLASH, FLASH_START},
    pac,
    prelude::*,
    rcc,
};

#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(rcc::Config::hsi16());
    let mut flash = FLASH::new(dp.FLASH, &mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    let mut led = gpiob.pb2.into_push_pull_output();

    // Get the delay provider.
    let mut delay = cp.SYST.delay(rcc.clocks);

    // This should be the first word in the second flash bank. Since this
    // example should be quite small, we can be reasonably sure that it fully
    // fits into the first flash bank. This means we won't overwrite our own
    // code or stall execution.
    //
    // This example requires STM32L082, which has 2 banks.
    let address = FLASH_START + flash_size_in_kb() / 2 * 1024;
    let address = address as *mut u32;

    flash
        .erase_flash_page(address)
        .expect("Failed to erase Flash page (1)");
    for i in 0..32 {
        let word = unsafe { *address.offset(i * 4) };
        assert_eq!(word, 0);
    }

    flash
        .write_word(address, 0x12345678)
        .expect("Failed to write word");
    assert_eq!(unsafe { *address }, 0x12345678);

    flash
        .erase_flash_page(address)
        .expect("Failed to erase Flash page (2)");
    for i in 0..32 {
        let word = unsafe { *address.offset(i * 4) };
        assert_eq!(word, 0);
    }

    let words = [
        0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf,
    ];

    flash
        .write_flash_half_page(address, &words)
        .expect("Failed to write Flash half-page");

    for (i, &expected) in words.iter().enumerate() {
        let actual = unsafe { *address.offset(i as isize) };
        assert_eq!(expected, actual);
    }

    // Blink LED to indicate we haven't panicked.
    loop {
        led.set_high().unwrap();
        delay.delay_ms(500_u16);

        led.set_low().unwrap();
        delay.delay_ms(500_u16);
    }
}
