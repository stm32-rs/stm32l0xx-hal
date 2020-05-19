//! Example showing continuous ADC with hardware trigger


#![no_main]
#![no_std]


extern crate panic_halt;


use core::{
    fmt::Write as _,
    pin::Pin,
};


use cortex_m_rt::entry;
use stm32l0xx_hal::{
    prelude::*,
    adc,
    dma::DMA,
    pac::{
        self,
        tim2::cr2::MMS_A,
    },
    rcc,
    serial,
};

use stm32l0xx_hal::serial::Serial1Ext;

const BUFSIZE : usize = 8; 	// the size of the buffer to use
const FREQUENCY : u32 = 1; // the frequency to sample at

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc   = dp.RCC.freeze(rcc::Config::hsi16());
    let     adc   = dp.ADC.constrain(&mut rcc);
    let mut dma   = DMA::new(dp.DMA1, &mut rcc);
    let     gpioa = dp.GPIOA.split(&mut rcc);
    let     gpiob = dp.GPIOB.split(&mut rcc);

    // The A0 connector on the B-L072Z-LRWAN1 Discovery kit
    let a0 = gpioa.pa5.into_analog();

    // Connected to the host computer via the ST-LINK
    let tx = gpiob.pb6;
    let rx = gpiob.pb7;

    // Initialize USART for test output
    let (mut tx, _) = dp.USART1
        .usart(
            tx, rx,
            serial::Config::default()
                .baudrate(115_200.bps()),
            &mut rcc,
        )
        .unwrap()
        .split();

    // Create the buffer we're going to use for DMA.
    //
    // This is safe, since this is the main function, and it's only executed
    // once. This means there is no other code accessing this `static`.   
    static mut BUFFER: [u16; BUFSIZE] = [0; BUFSIZE];
    let buffer = Pin::new(unsafe { &mut BUFFER });

    // Start reading ADC values
    let mut adc = adc.start(
        a0,
        Some(adc::Trigger::TIM2_TRGO),
        &mut dma.handle,
        dma.channels.channel1,
        buffer,
    );

    // Enable trigger output for TIM2. This must happen after ADC has been
    // configured.
    dp.TIM2
        .timer(FREQUENCY.hz(), &mut rcc)
        .select_master_mode(MMS_A::UPDATE);

    loop {
        for val in adc.read_available().unwrap() {
            write!(tx, "{}\r\n", val.unwrap()).unwrap();
        }
    }
}
