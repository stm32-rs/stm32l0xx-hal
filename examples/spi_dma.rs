#![no_main]
#![no_std]

extern crate panic_halt;

use core::pin::Pin;

use cortex_m::interrupt;
use cortex_m_rt::entry;
use stm32l0xx_hal::{
    dma::{self, DMA},
    pac::{self, Interrupt, NVIC},
    prelude::*,
    pwr::PWR,
    rcc::Config,
    spi,
};

#[entry]
fn main() -> ! {
    let cp = pac::CorePeripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut nss = gpioa.pa4.into_push_pull_output();
    let sck = gpioa.pa5;
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7;

    // Initialise the SPI peripheral.
    let mut spi = dp
        .SPI1
        .spi((sck, miso, mosi), spi::MODE_0, 100_000.Hz(), &mut rcc);

    let mut scb = cp.SCB;
    let mut dma = DMA::new(dp.DMA1, &mut rcc);
    let mut pwr = PWR::new(dp.PWR, &mut rcc);

    let mut tx_channel = dma.channels.channel3;

    // Create the buffer we're going to use for DMA.
    // This is safe, since this is the main function, and it's only executed
    // once. This means there is no other code accessing this `static`.
    static mut BUFFER: [u8; 1] = [0; 1];
    let mut buffer = Pin::new(unsafe { &mut BUFFER });

    loop {
        nss.set_low().unwrap();

        // Prepare requesting data from reference register
        let mut transfer = spi.write_all(&mut dma.handle, tx_channel, buffer);

        // Start DMA transfer and wait for it to finish
        let res = interrupt::free(|_| {
            unsafe {
                NVIC::unmask(Interrupt::DMA1_CHANNEL2_3);
            }

            transfer.enable_interrupts(dma::Interrupts {
                transfer_error: true,
                transfer_complete: true,
                ..dma::Interrupts::default()
            });

            let transfer = transfer.start();

            // Wait for the DMA transfer to finish. Since we first sleep until
            // an interrupt occurs, we know that the call to `wait` will return
            // immediately.
            pwr.sleep_mode(&mut scb).enter();
            let res = transfer.wait().unwrap();

            NVIC::mask(Interrupt::DMA1_CHANNEL2_3);
            res
        });

        spi = res.target;
        tx_channel = res.channel;
        buffer = res.buffer;

        nss.set_high().unwrap();
    }
}
