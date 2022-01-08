use core::fmt;
use core::marker::PhantomData;
use core::ptr;

use nb::block;

use crate::gpio::{AltMode, PinMode};
use crate::hal;
use crate::hal::prelude::*;
pub use crate::pac::{LPUART1, USART1, USART2, USART4, USART5};
use crate::rcc::{Enable, Rcc, LSE};
use embedded_time::rate::{Baud, Extensions};

#[cfg(any(feature = "stm32l0x1", feature = "stm32l0x2", feature = "stm32l0x3"))]
use core::{
    ops::{Deref, DerefMut},
    pin::Pin,
};

#[cfg(any(feature = "stm32l0x1", feature = "stm32l0x2", feature = "stm32l0x3"))]
use as_slice::{AsMutSlice, AsSlice};

#[cfg(any(feature = "stm32l0x1", feature = "stm32l0x2", feature = "stm32l0x3"))]
pub use crate::dma;

#[cfg(any(feature = "stm32l0x1", feature = "stm32l0x2", feature = "stm32l0x3"))]
use crate::dma::Buffer;

#[cfg(any(
    feature = "io-STM32L031",
    feature = "io-STM32L051",
    feature = "io-STM32L071"
))]
use crate::gpio::gpioc::*;
use crate::gpio::{gpioa::*, gpiob::*};
#[cfg(any(feature = "io-STM32L071"))]
use crate::gpio::{gpiod::*, gpioe::*};

/// Serial error
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Framing error
    Framing,
    /// Noise error
    Noise,
    /// RX buffer overrun
    Overrun,
    /// Parity check error
    Parity,
}

/// Interrupt event
pub enum Event {
    /// New data has been received.
    ///
    /// This event is cleared by reading a character from the UART.
    Rxne,
    /// New data can be sent.
    ///
    /// This event is cleared by writing a character to the UART.
    ///
    /// Note that this event does not mean that the character in the TX buffer
    /// is fully transmitted. It only means that the TX buffer is ready to take
    /// another character to be transmitted.
    Txe,
    /// Idle line state detected.
    Idle,
}

pub enum WordLength {
    DataBits8,
    DataBits9,
}

pub enum Parity {
    ParityNone,
    ParityEven,
    ParityOdd,
}

pub enum StopBits {
    #[doc = "1 stop bit"]
    STOP1,
    #[doc = "0.5 stop bits"]
    STOP0P5,
    #[doc = "2 stop bits"]
    STOP2,
    #[doc = "1.5 stop bits"]
    STOP1P5,
}

pub struct Config {
    pub baudrate: Baud,
    pub wordlength: WordLength,
    pub parity: Parity,
    pub stopbits: StopBits,
}

impl Config {
    pub fn baudrate(mut self, baudrate: impl Into<Baud>) -> Self {
        self.baudrate = baudrate.into();
        self
    }

    pub fn parity_none(mut self) -> Self {
        self.parity = Parity::ParityNone;
        self
    }

    pub fn parity_even(mut self) -> Self {
        self.parity = Parity::ParityEven;
        self
    }

    pub fn parity_odd(mut self) -> Self {
        self.parity = Parity::ParityOdd;
        self
    }

    pub fn wordlength_8(mut self) -> Self {
        self.wordlength = WordLength::DataBits8;
        self
    }

    pub fn wordlength_9(mut self) -> Self {
        self.wordlength = WordLength::DataBits9;
        self
    }

    pub fn stopbits(mut self, stopbits: StopBits) -> Self {
        self.stopbits = stopbits;
        self
    }
}

#[derive(Debug)]
pub struct InvalidConfig;

impl Default for Config {
    fn default() -> Config {
        let baudrate = 9_600_u32.Bd();
        Config {
            baudrate,
            wordlength: WordLength::DataBits8,
            parity: Parity::ParityNone,
            stopbits: StopBits::STOP1,
        }
    }
}

/// Trait to mark serial pins with transmit capability.
pub trait TxPin<USART> {
    fn setup(&self);
}

/// Trait to mark serial pins with receive capability.
pub trait RxPin<USART> {
    fn setup(&self);
}

/// Macro to implement `TxPin` / `RxPin` for a certain pin, using a certain
/// alternative function and for a certain serial peripheral.
macro_rules! impl_pins {
    ($($pin:ident, $alt:ident, $instance:ty, $trait:ident;)*) => {
        $(
            impl<MODE: PinMode> $trait<$instance> for $pin<MODE> {
                fn setup(&self) {
                    self.set_alt_mode(AltMode::$alt);
                }
            }
        )*
    }
}

#[cfg(feature = "io-STM32L021")]
impl_pins!(
    PA0, AF0, USART2, RxPin;
    PA0, AF6, LPUART1, RxPin;
    PA1, AF6, LPUART1, TxPin;
    PA2, AF4, USART2, TxPin;
    PA2, AF6, LPUART1, TxPin;
    PA3, AF4, USART2, RxPin;
    PA3, AF6, LPUART1, RxPin;
    PA4, AF6, LPUART1, TxPin;
    PA9, AF4, USART2, TxPin;
    PA10, AF4, USART2, RxPin;
    PA13, AF6, LPUART1, RxPin;
    PA14, AF4, USART2, TxPin;
    PA14, AF6, LPUART1, TxPin;
    PA15, AF4, USART2, RxPin;
    PB6, AF0, USART2, TxPin;
    PB6, AF6, LPUART1, TxPin;
    PB7, AF0, USART2, RxPin;
    PB7, AF6, LPUART1, RxPin;
    PB8, AF0, USART2, TxPin;
);

#[cfg(feature = "io-STM32L031")]
impl_pins!(
    PA2, AF4, USART2, TxPin;
    PA2, AF6, LPUART1, TxPin;
    PA3, AF4, USART2, RxPin;
    PA3, AF6, LPUART1, RxPin;
    PA9, AF4, USART2, TxPin;
    PA10, AF4, USART2, RxPin;
    PA13, AF6, LPUART1, RxPin;
    PA14, AF4, USART2, TxPin;
    PA14, AF6, LPUART1, TxPin;
    PA15, AF4, USART2, RxPin;
    PB6, AF0, USART2, TxPin;
    PB7, AF0, USART2, RxPin;
    PB10, AF6, LPUART1, TxPin;
    PB11, AF6, LPUART1, RxPin;
    PC0, AF6, LPUART1, RxPin;
);

#[cfg(feature = "io-STM32L051")]
impl_pins!(
    PA2, AF4, USART2, TxPin;
    PA3, AF4, USART2, RxPin;
    PA9, AF4, USART1, TxPin;
    PA10, AF4, USART1, RxPin;
    PA14, AF4, USART2, TxPin;
    PA15, AF4, USART2, RxPin;
    PB6, AF0, USART1, TxPin;
    PB7, AF0, USART1, RxPin;
    PB10, AF4, LPUART1, TxPin;
    PB11, AF4, LPUART1, RxPin;
    PC4, AF2, LPUART1, TxPin;
    PC5, AF2, LPUART1, RxPin;
    PC10, AF0, LPUART1, TxPin;
    PC11, AF0, LPUART1, RxPin;
);

#[cfg(feature = "io-STM32L071")]
impl_pins!(
    PA0, AF6, USART4, TxPin;
    PA1, AF6, USART4, RxPin;
    PA2, AF4, USART2, TxPin;
    PA2, AF6, LPUART1, TxPin;
    PA3, AF4, USART2, RxPin;
    PA3, AF6, LPUART1, RxPin;
    PA9, AF4, USART1, TxPin;
    PA10, AF4, USART1, RxPin;
    PA13, AF6, LPUART1, RxPin;
    PA14, AF4, USART2, TxPin;
    PA14, AF6, LPUART1, TxPin;
    PA15, AF4, USART2, RxPin;
    PB3, AF6, USART5, TxPin;
    PB4, AF6, USART5, RxPin;
    PB6, AF0, USART1, TxPin;
    PB7, AF0, USART1, RxPin;
    PB10, AF4, LPUART1, TxPin;
    PB10, AF7, LPUART1, RxPin;
    PB11, AF4, LPUART1, RxPin;
    PB11, AF7, LPUART1, TxPin;
    PC0, AF6, LPUART1, RxPin;
    PC1, AF6, LPUART1, TxPin;
    PC4, AF2, LPUART1, TxPin;
    PC5, AF2, LPUART1, RxPin;
    PC10, AF0, LPUART1, TxPin;
    PC10, AF6, USART4, TxPin;
    PC11, AF0, LPUART1, RxPin;
    PC11, AF6, USART4, RxPin;
    PC12, AF2, USART5, TxPin;
    PD2, AF6, USART5, RxPin;
    PD5, AF0, USART2, TxPin;
    PD6, AF0, USART2, RxPin;
    PD8, AF0, LPUART1, TxPin;
    PD9, AF0, LPUART1, RxPin;
    PE8, AF6, USART4, TxPin;
    PE9, AF6, USART4, RxPin;
    PE10, AF6, USART5, TxPin;
    PE11, AF6, USART5, RxPin;
);

/// Serial abstraction
pub struct Serial<USART> {
    usart: USART,
    rx: Rx<USART>,
    tx: Tx<USART>,
}

/// Serial receiver
pub struct Rx<USART> {
    _usart: PhantomData<USART>,
}

/// Serial transmitter
pub struct Tx<USART> {
    _usart: PhantomData<USART>,
}

macro_rules! usart {
    ($(
        $USARTX:ident: ($usartX:ident, $pclkX:ident, $SerialExt:ident),
    )+) => {
        $(
            pub trait $SerialExt<TX, RX> {
                fn usart(self, tx: TX, rx: RX, config: Config, rcc: &mut Rcc) -> Result<Serial<$USARTX>, InvalidConfig>;
            }

            impl<TX, RX> $SerialExt<TX, RX> for $USARTX
                where
                    TX: TxPin<$USARTX>,
                    RX: RxPin<$USARTX>,
            {
                fn usart(self, tx: TX, rx: RX, config: Config, rcc: &mut Rcc) -> Result<Serial<$USARTX>, InvalidConfig> {
                    Serial::$usartX(self, tx, rx, config, rcc)
                }
            }

            impl Serial<$USARTX> {
                pub fn $usartX<TX, RX>(
                    usart: $USARTX,
                    tx: TX,
                    rx: RX,
                    config: Config,
                    rcc: &mut Rcc,
                ) -> Result<Self, InvalidConfig>
                where
                    TX: TxPin<$USARTX>,
                    RX: RxPin<$USARTX>,
                {
                    tx.setup();
                    rx.setup();

                    // Enable clock for USART
                    <$USARTX>::enable(rcc);

                    // Calculate correct baudrate divisor on the fly
                    let div = (rcc.clocks.$pclkX().0 * 25) / (4 * config.baudrate.0);
                    let mantissa = div / 100;
                    let fraction = ((div - mantissa * 100) * 16 + 50) / 100;
                    let mut brr = mantissa << 4 | fraction;

                    if stringify!($usartX) == "lpuart1" {
                        brr *= 256
                    }

                    usart
                        .brr
                        .write(|w| unsafe { w.bits(brr) });

                    // Reset other registers to disable advanced USART features
                    usart.cr2.reset();

                    // Enable DMA
                    usart.cr3.write(|w|
                        w
                            // Stop DMA transfer on reception error
                            .ddre().disabled()
                            // Enable DMA
                            .dmat().enabled()
                            .dmar().enabled()
                    );

                    // Enable transmission and receiving
                    // and configure frame
                    usart.cr1.write(|w| {
                        w.ue()
                            .set_bit()
                            .te()
                            .set_bit()
                            .re()
                            .set_bit()
                            .m0()
                            .bit(match config.wordlength {
                                WordLength::DataBits8 => false,
                                WordLength::DataBits9 => true,
                            }).pce()
                            .bit(match config.parity {
                                Parity::ParityNone => false,
                                _ => true,
                            }).ps()
                            .bit(match config.parity {
                                Parity::ParityOdd => true,
                                _ => false,
                            })
                    });

                    usart.cr2.write(|w|
                        w.stop().bits(match config.stopbits {
                            StopBits::STOP1 => 0b00,
                            StopBits::STOP0P5 => 0b01,
                            StopBits::STOP2 => 0b10,
                            StopBits::STOP1P5 => 0b11,
                        })
                    );
                    Ok(Serial {
                        usart,
                        tx: Tx { _usart: PhantomData },
                        rx: Rx { _usart: PhantomData },
                    })
                }

                /// Starts listening for an interrupt event
                pub fn listen(&mut self, event: Event) {
                    match event {
                        Event::Rxne => {
                            self.usart.cr1.modify(|_, w| w.rxneie().set_bit())
                        },
                        Event::Txe => {
                            self.usart.cr1.modify(|_, w| w.txeie().set_bit())
                        },
                        Event::Idle => {
                            self.usart.cr1.modify(|_, w| w.idleie().set_bit())
                        },
                    }
                }

                /// Stop listening for an interrupt event
                pub fn unlisten(&mut self, event: Event) {
                    match event {
                        Event::Rxne => {
                            self.usart.cr1.modify(|_, w| w.rxneie().clear_bit())
                        },
                        Event::Txe => {
                            self.usart.cr1.modify(|_, w| w.txeie().clear_bit())
                        },
                        Event::Idle => {
                            self.usart.cr1.modify(|_, w| w.idleie().clear_bit())
                        },
                    }
                }

                /// Returns a pending and enabled `Event`.
                ///
                /// Multiple `Event`s can be signaled at the same time. In that case, an arbitrary
                /// pending event will be returned. Clearing the event condition will cause this
                /// method to return the other pending event(s).
                ///
                /// For an event to be returned by this method, it must first be enabled by calling
                /// `listen`.
                ///
                /// This method will never clear a pending event. If the event condition is not
                /// resolved by the user, it will be returned again by the next call to
                /// `pending_event`.
                pub fn pending_event(&self) -> Option<Event> {
                    let cr1 = self.usart.cr1.read();
                    let isr = self.usart.isr.read();

                    if cr1.rxneie().bit_is_set() && isr.rxne().bit_is_set() {
                        // Give highest priority to RXNE to help with avoiding overrun
                        Some(Event::Rxne)
                    } else if cr1.txeie().bit_is_set() && isr.txe().bit_is_set() {
                        Some(Event::Txe)
                    } else if cr1.idleie().bit_is_set() && isr.idle().bit_is_set() {
                        Some(Event::Idle)
                    } else {
                        None
                    }
                }

                /// Checks for reception errors that may have occurred.
                ///
                /// Note that multiple errors can be signaled at the same time. In that case,
                /// calling this function repeatedly will return the remaining errors.
                pub fn check_errors(&mut self) -> Result<(), Error> {
                    self.rx.check_errors()
                }

                /// Clears any signaled errors without returning them.
                pub fn clear_errors(&mut self) {
                    self.rx.clear_errors()
                }

                pub fn split(self) -> (Tx<$USARTX>, Rx<$USARTX>) {
                    (self.tx, self.rx)
                }

                pub fn release(self) -> $USARTX {
                    self.usart
                }
            }

            impl hal::serial::Read<u8> for Serial<$USARTX> {
                type Error = Error;

                fn read(&mut self) -> nb::Result<u8, Error> {
                    self.rx.read()
                }
            }

             impl hal::serial::Write<u8> for  Serial<$USARTX> {
                type Error = Error;

                fn flush(&mut self) -> nb::Result<(), Self::Error> {
                    self.tx.flush()
                }

                fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
                    self.tx.write(byte)
                }
            }

            impl Rx<$USARTX> {
                /// Returns true if the line idle status is set
                /// This reads the ISR register's IDLE bit. This bit is set by hardware
                /// when an Idle Line is detected. And can be cleared by calling `clear_idle_interrupt`.
                ///
                /// This flag is set by hardware even when interrupts are disabled (IDLEIE=0 in CR1)
                pub fn is_idle(&self) -> bool {
                    let isr = unsafe { (*$USARTX::ptr()).isr.read() };
                    isr.idle().bit_is_set()
                }

                /// Returns true if the rx register is not empty (and can be read)
                pub fn is_rx_not_empty(&self) -> bool {
                    let isr = unsafe { (*$USARTX::ptr()).isr.read() };
                    isr.rxne().bit_is_set()
                }

                /// Clear idle line interrupt flag
                pub fn clear_idle_interrupt(&self) {
                    let icr = unsafe { &(*$USARTX::ptr()).icr };
                    icr.write(|w| w.idlecf().set_bit());
                }

                /// Checks for reception errors that may have occurred.
                ///
                /// Note that multiple errors can be signaled at the same time. In that case,
                /// calling this function repeatedly will return the remaining errors.
                pub fn check_errors(&mut self) -> Result<(), Error> {
                    let isr = unsafe { (*$USARTX::ptr()).isr.read() };
                    let icr = unsafe { &(*$USARTX::ptr()).icr };

                    // We don't want to drop any errors, so check each error bit in sequence. If
                    // any bit is set, clear it and return its error.
                    if isr.pe().bit_is_set() {
                        icr.write(|w| {w.pecf().set_bit()});
                        return Err(Error::Parity.into());
                    } else if isr.fe().bit_is_set() {
                        icr.write(|w| {w.fecf().set_bit()});
                        return Err(Error::Framing.into());
                    } else if isr.nf().bit_is_set() {
                        icr.write(|w| {w.ncf().set_bit()});
                        return Err(Error::Noise.into());
                    } else if isr.ore().bit_is_set() {
                        icr.write(|w| {w.orecf().set_bit()});
                        return Err(Error::Overrun.into());
                    }

                    Ok(())
                }

                /// Clears any signaled errors without returning them.
                pub fn clear_errors(&mut self) {
                    let icr = unsafe { &(*$USARTX::ptr()).icr };

                    icr.write(|w| w
                        .pecf().set_bit()
                        .fecf().set_bit()
                        .ncf().set_bit()
                        .orecf().set_bit()
                    );
                }
            }

            /// DMA operations.
            #[cfg(any(feature = "stm32l0x1", feature = "stm32l0x2", feature = "stm32l0x3"))]
            impl Rx<$USARTX> {
                pub fn read_all<Buffer, Channel>(self,
                    dma:     &mut dma::Handle,
                    buffer:  Pin<Buffer>,
                    channel: Channel,
                )
                    -> dma::Transfer<Self, Channel, Buffer, dma::Ready>
                    where
                        Self:           dma::Target<Channel>,
                        Buffer:         DerefMut + 'static,
                        Buffer::Target: AsMutSlice<Element=u8>,
                        Channel:        dma::Channel,
                {
                    let num_words = (*buffer).len();
                    self.read_some(dma, buffer, num_words, channel)
                }

                pub fn read_some<Buffer, Channel>(self,
                    dma:     &mut dma::Handle,
                    buffer:  Pin<Buffer>,
                    num_words: usize,
                    channel: Channel,
                )
                    -> dma::Transfer<Self, Channel, Buffer, dma::Ready>
                    where
                        Self:           dma::Target<Channel>,
                        Buffer:         DerefMut + 'static,
                        Buffer::Target: AsMutSlice<Element=u8>,
                        Channel:        dma::Channel,
                {
                    // Safe, because we're only taking the address of a
                    // register.
                    let address =
                        &unsafe { &*$USARTX::ptr() }.rdr as *const _ as u32;

                    // Safe, because the trait bounds of this method guarantee
                    // that the buffer can be written to.
                    unsafe {
                        dma::Transfer::new(
                            dma,
                            self,
                            channel,
                            buffer,
                            num_words,
                            address,
                            dma::Priority::high(),
                            dma::Direction::peripheral_to_memory(),
                            false,
                        )
                    }
                }
            }

            impl hal::serial::Read<u8> for Rx<$USARTX> {
                type Error = Error;

                fn read(&mut self) -> nb::Result<u8, Error> {
                    self.check_errors()?;

                    // NOTE(unsafe) atomic read with no side effects
                    let isr = unsafe { (*$USARTX::ptr()).isr.read() };

                    // Check if a byte is available
                    if isr.rxne().bit_is_set() {
                        // Read the received byte
                        // NOTE(read_volatile) see `write_volatile` below
                        Ok(unsafe {
                            ptr::read_volatile(&(*$USARTX::ptr()).rdr as *const _ as *const _)
                        })
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                }
            }

            impl hal::serial::Write<u8> for Tx<$USARTX> {
                type Error = Error;

                fn flush(&mut self) -> nb::Result<(), Self::Error> {
                    // NOTE(unsafe) atomic read with no side effects
                    let isr = unsafe { (*$USARTX::ptr()).isr.read() };

                    // Check TC bit on ISR
                    if isr.tc().bit_is_set() {
                        // Frame complete, set the TC Clear Flag
                        unsafe {
                            (*$USARTX::ptr()).icr.write(|w| {w.tccf().set_bit()});
                        }
                        Ok(())
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                }

                fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
                    // NOTE(unsafe) atomic read with no side effects
                    let isr = unsafe { (*$USARTX::ptr()).isr.read() };

                    if isr.txe().bit_is_set() {
                        // NOTE(unsafe) atomic write to stateless register
                        // NOTE(write_volatile) 8-bit write that's not possible through the svd2rust API
                        unsafe { ptr::write_volatile(&(*$USARTX::ptr()).tdr as *const _ as *mut _, byte) }

                        Ok(())
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                }
            }

            impl Tx<$USARTX> {
                /// Returns true if the tx register is empty (and can accept data)
                pub fn is_tx_empty(&self) -> bool {
                    let isr = unsafe { (*$USARTX::ptr()).isr.read() };
                    isr.txe().bit_is_set()
                }
            }

            #[cfg(any(feature = "stm32l0x1", feature = "stm32l0x2", feature = "stm32l0x3"))]
            impl Tx<$USARTX> {
                pub fn write_all<Buffer, Channel>(self,
                    dma:     &mut dma::Handle,
                    buffer:  Pin<Buffer>,
                    channel: Channel,
                )
                    -> dma::Transfer<Self, Channel, Buffer, dma::Ready>
                    where
                        Self:           dma::Target<Channel>,
                        Buffer:         Deref + 'static,
                        Buffer::Target: AsSlice<Element=u8>,
                        Channel:        dma::Channel,
                {
                    let num_words = (*buffer).len();
                    self.write_some(dma, buffer, num_words, channel)
                }

                pub fn write_some<Buffer, Channel>(self,
                    dma:     &mut dma::Handle,
                    buffer:  Pin<Buffer>,
                    num_words:  usize,
                    channel: Channel,
                )
                    -> dma::Transfer<Self, Channel, Buffer, dma::Ready>
                    where
                        Self:           dma::Target<Channel>,
                        Buffer:         Deref + 'static,
                        Buffer::Target: AsSlice<Element=u8>,
                        Channel:        dma::Channel,
                {
                    // Safe, because we're only taking the address of a
                    // register.
                    let address =
                        &unsafe { &*$USARTX::ptr() }.tdr as *const _ as u32;

                    // Safe, because the trait bounds of this method guarantee
                    // that the buffer can be read from.
                    unsafe {
                        dma::Transfer::new(
                            dma,
                            self,
                            channel,
                            buffer,
                            num_words,
                            address,
                            dma::Priority::high(),
                            dma::Direction::memory_to_peripheral(),
                            false,
                        )
                    }
                }
            }
        )+
    }
}

// LPUART1 and USART2 are available on category 1/2/3/5 MCUs
#[cfg(any(
    feature = "io-STM32L021",
    feature = "io-STM32L031",
    feature = "io-STM32L051",
    feature = "io-STM32L071",
))]
usart! {
    LPUART1: (lpuart1, apb1_clk, Serial1LpExt),
    USART2: (usart2, apb1_clk, Serial2Ext),
}

// USART1 is available on category 3/5 MCUs
#[cfg(any(feature = "io-STM32L051", feature = "io-STM32L071"))]
usart! {
    USART1: (usart1, apb1_clk, Serial1Ext),
}

// USART4 and USART5 are available on category 5 MCUs
#[cfg(feature = "io-STM32L071")]
usart! {
    USART4: (usart4, apb1_clk, Serial4Ext),
    USART5: (usart5, apb1_clk, Serial5Ext),
}

impl Serial<LPUART1> {
    /// Switches LPUART1 clock course to LSE
    ///
    /// Consumes LSE token, to get guarantee that
    /// LSE clocks are configured.
    ///
    /// At the moment configured baudrate is ignored
    /// and LPUART1 is forced to 9600 when clocked by LSE,
    /// assuming that LSE is 32768
    /// (and it must be so according to RM).
    pub fn use_lse(&mut self, rcc: &mut Rcc, _: &LSE) {
        //Disable transmitter
        self.usart.cr1.modify(|_, w| w.te().disabled());
        while self.usart.isr.read().tc().bit_is_clear() {}

        //Disable LPUART1
        self.usart.cr1.modify(|_, w| w.ue().disabled());

        //Reconfigure LPUART to use LSE
        rcc.rb.ccipr.modify(|_, w| w.lpuart1sel().lse());

        //Recalculate baudrate
        //TODO requested baudrate value from the config should be stored somehow and used here
        //but at the moment hardcoded 9600 will be used
        //LSE is assumed to be 32768Hz, as RM says that LSE should only be 32768.
        let brr = 256 * 32768 / 9600;
        self.usart.brr.write(|w| unsafe { w.bits(brr) });

        // Enable LPUART1
        self.usart
            .cr1
            .write(|w| w.ue().set_bit().te().set_bit().re().set_bit());
    }
}

impl<USART> fmt::Write for Serial<USART>
where
    Serial<USART>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = s.as_bytes().iter().map(|c| block!(self.write(*c))).last();

        //self.flush().map_err(|_| fmt::Error)?;

        Ok(())
    }
}

impl<USART> fmt::Write for Tx<USART>
where
    Tx<USART>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = s.as_bytes().iter().map(|c| block!(self.write(*c))).last();

        //self.flush().map_err(|_| fmt::Error)?;

        Ok(())
    }
}
