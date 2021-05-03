#![deny(unsafe_code)]
#![no_std]
#![no_main]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{crc::*, pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let crc = dp.CRC.constrain(&mut rcc);

    // Lets use the Ethernet CRC. The polynomial is there by default
    // but we need reflected in (by byte) and reflected out.
    let mut crc = crc
        .input_bit_reversal(BitReversal::ByByte)
        .output_bit_reversal(true)
        .freeze();

    let data = [
        0x21, 0x10, 0x00, 0x00, 0x63, 0x30, 0x42, 0x20, 0xa5, 0x50, 0x84, 0x40, 0xe7, 0x70, 0xc6,
        0x60, 0x4a, 0xa1, 0x29, 0x91, 0x8c, 0xc1, 0x6b, 0xb1, 0xce, 0xe1, 0xad, 0xd1, 0x31, 0x12,
        0xef, 0xf1, 0x52, 0x22, 0x73, 0x32, 0xa1, 0xb2, 0xc3,
    ];

    crc.feed(&data);

    // CRC32 has final XOR value of 0xFFFFFFFF
    let result = crc.result() ^ 0xffff_ffff;

    // check against https://crccalc.com/
    // with 2110000063304220a5508440e770c6604aa129918cc16bb1cee1add13112eff152227332a1b2c3
    // as hex input
    assert!(result == 0x5C45_81C0);

    loop {}
}
