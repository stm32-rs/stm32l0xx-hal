//! Encryption/decryption using the AES peripheral

#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::pin::Pin;

use aligned::{Aligned, A4};
use cortex_m::{asm, interrupt, peripheral::NVIC};
use cortex_m_rt::entry;
use stm32l0xx_hal::{
    aes::{self, AES},
    dma::{self, DMA},
    pac::{self, Interrupt},
    prelude::*,
    rcc::Config,
};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut aes = AES::new(dp.AES, &mut rcc);
    let mut dma = DMA::new(dp.DMA1, &mut rcc);

    let key = [0x01234567, 0x89abcdef, 0x01234567, 0x89abcdef];
    let ivr = [0xfedcba98, 0x76543210, 0xfedcba98];

    const DATA: Aligned<A4, [u8; 32]> = Aligned([
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
        0xee, 0xff,
    ]);
    let data = Pin::new(&DATA);

    static mut ENCRYPTED: Aligned<A4, [u8; 32]> = Aligned([0; 32]);
    static mut DECRYPTED: Aligned<A4, [u8; 32]> = Aligned([0; 32]);

    // Prepare DMA buffers. This is safe, as this is the `main` function, and no
    // other functions have access to these statics.
    let mut encrypted = Pin::new(unsafe { &mut ENCRYPTED });
    let mut decrypted = Pin::new(unsafe { &mut DECRYPTED });

    loop {
        let mut ctr_stream = aes.enable(aes::Mode::ctr(ivr), key);
        let mut tx_transfer = ctr_stream
            .tx
            .write_all(&mut dma.handle, data, dma.channels.channel1);
        let mut rx_transfer =
            ctr_stream
                .rx
                .read_all(&mut dma.handle, encrypted, dma.channels.channel2);

        let (tx_res, rx_res) = interrupt::free(|_| {
            unsafe {
                NVIC::unmask(Interrupt::DMA1_CHANNEL1);
            }
            unsafe {
                NVIC::unmask(Interrupt::DMA1_CHANNEL2_3);
            }

            let interrupts = dma::Interrupts {
                transfer_error: true,
                transfer_complete: true,
                ..Default::default()
            };

            tx_transfer.enable_interrupts(interrupts);
            rx_transfer.enable_interrupts(interrupts);

            let tx_transfer = tx_transfer.start();
            let rx_transfer = rx_transfer.start();

            asm::wfi();

            let tx_res = tx_transfer.wait().unwrap();
            let rx_res = rx_transfer.wait().unwrap();

            NVIC::mask(Interrupt::DMA1_CHANNEL1);
            NVIC::mask(Interrupt::DMA1_CHANNEL2_3);

            (tx_res, rx_res)
        });

        ctr_stream.tx = tx_res.target;
        ctr_stream.rx = rx_res.target;
        dma.channels.channel1 = tx_res.channel;
        dma.channels.channel2 = rx_res.channel;
        encrypted = rx_res.buffer;
        aes = ctr_stream.disable();

        assert_ne!(**encrypted, [0; 32]);
        assert_ne!(**encrypted, **data);

        let mut ctr_stream = aes.enable(aes::Mode::ctr(ivr), key);
        let mut tx_transfer =
            ctr_stream
                .tx
                .write_all(&mut dma.handle, encrypted, dma.channels.channel1);
        let mut rx_transfer =
            ctr_stream
                .rx
                .read_all(&mut dma.handle, decrypted, dma.channels.channel2);

        let (tx_res, rx_res) = interrupt::free(|_| {
            unsafe {
                NVIC::unmask(Interrupt::DMA1_CHANNEL1);
            }
            unsafe {
                NVIC::unmask(Interrupt::DMA1_CHANNEL2_3);
            }

            let interrupts = dma::Interrupts {
                transfer_error: true,
                transfer_complete: true,
                ..Default::default()
            };

            tx_transfer.enable_interrupts(interrupts);
            rx_transfer.enable_interrupts(interrupts);

            let tx_transfer = tx_transfer.start();
            let rx_transfer = rx_transfer.start();

            asm::wfi();

            let tx_res = tx_transfer.wait().unwrap();
            let rx_res = rx_transfer.wait().unwrap();

            NVIC::mask(Interrupt::DMA1_CHANNEL1);
            NVIC::mask(Interrupt::DMA1_CHANNEL2_3);

            (tx_res, rx_res)
        });

        ctr_stream.tx = tx_res.target;
        ctr_stream.rx = rx_res.target;
        dma.channels.channel1 = tx_res.channel;
        dma.channels.channel2 = rx_res.channel;
        encrypted = tx_res.buffer;
        decrypted = rx_res.buffer;
        aes = ctr_stream.disable();

        assert_eq!(**decrypted, **data);
    }
}
