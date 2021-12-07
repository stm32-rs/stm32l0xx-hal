//! Encryption/decryption using the AES peripheral

#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m_rt::entry;
use stm32l0xx_hal::{
    aes::{self, AES},
    pac,
    prelude::*,
    rcc::Config,
};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut aes = AES::new(dp.AES, &mut rcc);

    let key = [0x01234567, 0x89abcdef, 0x01234567, 0x89abcdef];
    let ivr = [0xfedcba98, 0x76543210, 0xfedcba98];

    let data = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ];

    loop {
        let mut ctr_stream = aes.enable(aes::Mode::ctr(ivr), key);

        let mut encrypted = [[0; 16]; 4];
        encrypted[0] = ctr_stream.process(&data).unwrap();
        encrypted[1] = ctr_stream.process(&data).unwrap();
        encrypted[2] = ctr_stream.process(&data).unwrap();
        encrypted[3] = ctr_stream.process(&data).unwrap();

        assert_ne!(encrypted[0], data);
        assert_ne!(encrypted[1], data);
        assert_ne!(encrypted[2], data);
        assert_ne!(encrypted[3], data);

        aes = ctr_stream.disable();
        let mut ctr_stream = aes.enable(aes::Mode::ctr(ivr), key);

        let mut decrypted = [[0; 16]; 4];
        decrypted[0] = ctr_stream.process(&encrypted[0]).unwrap();
        decrypted[1] = ctr_stream.process(&encrypted[1]).unwrap();
        decrypted[2] = ctr_stream.process(&encrypted[2]).unwrap();
        decrypted[3] = ctr_stream.process(&encrypted[3]).unwrap();

        assert_eq!(decrypted[0], data);
        assert_eq!(decrypted[1], data);
        assert_eq!(decrypted[2], data);
        assert_eq!(decrypted[3], data);

        aes = ctr_stream.disable();
    }
}
