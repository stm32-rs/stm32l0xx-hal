#![no_main]
#![no_std]

extern crate panic_halt;

use core::pin::Pin;

use cortex_m::{asm, interrupt, peripheral::NVIC};
use cortex_m_rt::entry;
use stm32l0xx_hal::{
    dma::{self, DMA},
    pac::{self, Interrupt},
    prelude::*,
    rcc::Config,
    serial,
};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut dma = DMA::new(dp.DMA1, &mut rcc);
    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut tx_channel = dma.channels.channel4;
    let mut rx_channel = dma.channels.channel5;

    let (mut tx, mut rx) = dp
        .USART2
        .usart(
            gpioa.pa2,
            gpioa.pa3,
            serial::Config::default().baudrate(115_200.Bd()),
            &mut rcc,
        )
        .unwrap()
        .split();

    // Create the buffer we're going to use for DMA.
    // This is safe, since this is the main function, and it's only executed
    // once. This means there is no other code accessing this `static`.
    static mut BUFFER: [u8; 1] = [0; 1];
    let mut buffer = Pin::new(unsafe { &mut BUFFER });

    loop {
        // Prepare read transfer
        let mut transfer = rx.read_all(&mut dma.handle, buffer, rx_channel);

        // Start DMA transfer and wait for it to finish
        let res = interrupt::free(|_| {
            unsafe {
                NVIC::unmask(Interrupt::DMA1_CHANNEL4_7);
            }

            transfer.enable_interrupts(dma::Interrupts {
                transfer_error: true,
                transfer_complete: true,
                ..dma::Interrupts::default()
            });

            let transfer = transfer.start();

            asm::wfi();
            let res = transfer.wait().unwrap();
            NVIC::mask(Interrupt::DMA1_CHANNEL4_7);

            res
        });

        // Re-assign reception resources to their variables, so they're
        // available again in the next loop iteration.
        rx = res.target;
        rx_channel = res.channel;
        buffer = res.buffer;

        // Prepare write transfer
        let mut transfer = tx.write_all(&mut dma.handle, buffer, tx_channel);

        // Start DMA transfer and wait for it to finish
        let res = interrupt::free(|_| {
            unsafe {
                NVIC::unmask(Interrupt::DMA1_CHANNEL4_7);
            }

            transfer.enable_interrupts(dma::Interrupts {
                transfer_error: true,
                transfer_complete: true,
                ..dma::Interrupts::default()
            });

            let transfer = transfer.start();

            asm::wfi();
            let res = transfer.wait().unwrap();
            NVIC::mask(Interrupt::DMA1_CHANNEL4_7);

            res
        });

        // Re-assign transmission resources to their variables, so they're
        // available again in the next loop iteration.
        tx = res.target;
        tx_channel = res.channel;
        buffer = res.buffer;
    }
}
