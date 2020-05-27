//! I2C example with DMA
//!
//! Uses an ST VL53L0X ToF sensor.

#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::pin::Pin;

use cortex_m::interrupt;
use cortex_m_rt::entry;
use stm32l0xx_hal::{
    dma::{self, DMA},
    pac::{self, Interrupt, NVIC},
    prelude::*,
    pwr::PWR,
    rcc::Config,
};

#[entry]
fn main() -> ! {
    let cp = pac::CorePeripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let mut scb = cp.SCB;
    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut dma = DMA::new(dp.DMA1, &mut rcc);
    let mut delay = cp.SYST.delay(rcc.clocks);
    let mut pwr = PWR::new(dp.PWR, &mut rcc);

    let gpiob = dp.GPIOB.split(&mut rcc);

    let sda = gpiob.pb9.into_open_drain_output();
    let scl = gpiob.pb8.into_open_drain_output();

    let mut green = gpiob.pb5.into_push_pull_output();
    let mut red = gpiob.pb7.into_push_pull_output();

    let mut i2c = dp.I2C1.i2c(sda, scl, 100.khz(), &mut rcc);

    let mut tx_channel = dma.channels.channel2;
    let mut rx_channel = dma.channels.channel3;

    // Create the buffer we're going to use for DMA.
    // This is safe, since this is the main function, and it's only executed
    // once. This means there is no other code accessing this `static`.
    static mut BUFFER: [u8; 1] = [0; 1];
    let mut buffer = Pin::new(unsafe { &mut BUFFER });

    let address = 0x52 >> 1;

    loop {
        buffer[0] = 0xc0; // address of on of the reference registers

        // Prepare requesting data from reference register
        let mut transfer = i2c.write_all(&mut dma.handle, tx_channel, address, buffer);

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

        i2c = res.target;
        tx_channel = res.channel;
        buffer = res.buffer;

        // Prepare to read returned data.
        let mut transfer = i2c.read_all(&mut dma.handle, rx_channel, address, buffer);

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

        i2c = res.target;
        rx_channel = res.channel;
        buffer = res.buffer;

        if buffer[0] == 0xee {
            green.set_high().unwrap();
            red.set_low().unwrap();
        } else {
            red.set_high().unwrap();
            green.set_low().unwrap();
        }

        delay.delay_ms(50u32);

        green.set_low().unwrap();
        red.set_low().unwrap();

        delay.delay_ms(50u32);
    }
}
