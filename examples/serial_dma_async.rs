#![no_main]
#![no_std]

extern crate panic_halt;

use core::pin::Pin;
use cortex_m_rt::entry;
use heapless::spsc::Queue;
use stm32l0xx_hal::{
    dma::{self, DMA},
    pac,
    prelude::*,
    rcc::Config,
    serial,
};

static mut BUFFER_1: [u8; 1] = [0; 1];
static mut BUFFER_2: [u8; 1] = [0; 1];
type DmaBuffer = &'static mut [u8; 1];

type RxTarget = serial::Rx<serial::USART2>;
type RxChannel = dma::Channel5;
type RxTransfer = dma::Transfer<RxTarget, RxChannel, DmaBuffer, dma::Started>;

enum RxState {
    READY(RxTarget, RxChannel),
    RECEIVING(RxTransfer),
}

type TxTarget = serial::Tx<serial::USART2>;
type TxChannel = dma::Channel4;
type TxTransfer = dma::Transfer<TxTarget, TxChannel, DmaBuffer, dma::Started>;

enum TxState {
    READY(TxTarget, TxChannel),
    TRANSMITTING(TxTransfer),
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi16());
    let mut dma = DMA::new(dp.DMA1, &mut rcc);
    let gpioa = dp.GPIOA.split(&mut rcc);

    let (tx, rx) = dp
        .USART2
        .usart(
            gpioa.pa2,
            gpioa.pa3,
            serial::Config::default().baudrate(115_200.Bd()),
            &mut rcc,
        )
        .unwrap()
        .split();

    // we only have two elements for each queue, so size is max 2 is fine.
    // Note that due to current const generics limitations the queue capacity
    // is calculated by heapless as N-1, so we declare it as '3-1' elements.
    let mut rx_buffers: Queue<Pin<DmaBuffer>, 3> = Queue::new();
    let mut tx_buffers: Queue<Pin<DmaBuffer>, 3> = Queue::new();

    // enqueue as many buffers as available into rx_buffers
    unsafe {
        // putting the same pointer in here twice would be a big mistake
        rx_buffers.enqueue(Pin::new(&mut BUFFER_1)).unwrap();
        rx_buffers.enqueue(Pin::new(&mut BUFFER_2)).unwrap();
    }

    let dma_handle = &mut dma.handle;
    let mut rx_state = RxState::READY(rx, dma.channels.channel5);
    let mut tx_state = TxState::READY(tx, dma.channels.channel4);

    loop {
        rx_state = match rx_state {
            RxState::READY(rx, channel) => {
                if let Some(buffer) = rx_buffers.dequeue() {
                    // prepare transfer transaction
                    let mut transfer = rx.read_all(dma_handle, buffer, channel);
                    transfer.enable_interrupts(dma::Interrupts {
                        transfer_error: true,
                        transfer_complete: true,
                        ..dma::Interrupts::default()
                    });
                    // store it in state
                    RxState::RECEIVING(transfer.start())
                } else {
                    panic!("Not enough buffers allocated\r");
                }
            }
            RxState::RECEIVING(transfer) => {
                if !transfer.is_active() {
                    let res = transfer.wait().unwrap();
                    // pass the buffer to the tx queue
                    tx_buffers.enqueue(res.buffer).unwrap();
                    // set back ready state
                    RxState::READY(res.target, res.channel)
                // next loop will setup next receive DMA
                } else {
                    RxState::RECEIVING(transfer)
                }
            }
        };

        tx_state = match tx_state {
            TxState::READY(tx, channel) => {
                if let Some(buffer) = tx_buffers.dequeue() {
                    // prepare transfer transaction
                    let mut transfer = tx.write_all(dma_handle, buffer, channel);
                    transfer.enable_interrupts(dma::Interrupts {
                        transfer_error: true,
                        transfer_complete: true,
                        ..dma::Interrupts::default()
                    });
                    // store it in state
                    TxState::TRANSMITTING(transfer.start())
                } else {
                    TxState::READY(tx, channel)
                }
            }
            TxState::TRANSMITTING(transfer) => {
                if !transfer.is_active() {
                    let res = transfer.wait().unwrap();
                    // give the buffer back to the rx_buffer queue
                    rx_buffers.enqueue(res.buffer).unwrap();
                    // set self back to ready state
                    TxState::READY(res.target, res.channel)
                // next loop around will check for another buffer
                } else {
                    TxState::TRANSMITTING(transfer)
                }
            }
        };
    }
}
