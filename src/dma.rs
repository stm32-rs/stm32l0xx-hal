//! Interface to the DMA peripheral
//!
//! See STM32L0x2 Reference Manual, chapter 11.

// Currently the only module using DMA is STM32L082-only, which leads to unused
// warnings when compiling for STM32L0x2. Rather than making the whole DMA
// module STM32L082-only, which wouldn't reflect the reality, I've added this
// stopgap measure to silence the warnings.
//
// This should be removed once there are any STM32L0x2 modules making use of
// DMA.
#![cfg_attr(not(feature = "stm32l082"), allow(dead_code, unused_imports))]

use core::{
    fmt, mem,
    ops::Deref,
    pin::Pin,
    sync::atomic::{compiler_fence, Ordering},
};

use as_slice::AsSlice;

use crate::{
    adc,
    pac::{self, dma1::ch::cr},
    rcc::{Enable, Rcc, Reset},
};

#[cfg(any(feature = "io-STM32L051", feature = "io-STM32L071"))]
use crate::pac::USART1;

#[cfg(any(
    feature = "io-STM32L021",
    feature = "io-STM32L031",
    feature = "io-STM32L051",
    feature = "io-STM32L071",
))]
use crate::{
    i2c,
    pac::{I2C1, I2C2, I2C3, USART2},
    serial,
};

use crate::{pac::SPI1, spi};

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
use crate::pac::SPI2;

#[cfg(feature = "stm32l082")]
use crate::aes;

/// Entry point to the DMA API
pub struct DMA {
    /// Handle to the DMA peripheral
    pub handle: Handle,

    /// DMA channels
    pub channels: Channels,
}

impl DMA {
    /// Create an instance of the DMA API
    pub fn new(dma: pac::DMA1, rcc: &mut Rcc) -> Self {
        // Enable peripheral clock
        pac::DMA1::enable(rcc);
        // Reset peripheral
        pac::DMA1::reset(rcc);

        Self {
            handle: Handle { dma },
            channels: Channels::new(),
        }
    }
}

/// Handle to the DMA peripheral
pub struct Handle {
    dma: pac::DMA1,
}

pub struct Transfer<T, C, B, State> {
    res: TransferResources<T, C, B>,
    _state: State,
}

pub type TransferResourcesResult<Target, Channel, Buffer> = Result<
    TransferResources<Target, Channel, Buffer>,
    (TransferResources<Target, Channel, Buffer>, Error),
>;

impl<T, C, B> Transfer<T, C, B, Ready>
where
    T: Target<C>,
    C: Channel,
{
    #![allow(clippy::too_many_arguments)]
    /// Internal constructor
    ///
    /// # Safety
    ///
    /// If this is used to prepare a memory-to-peripheral transfer, the caller
    /// must make sure that the buffer can be read from safely.
    ///
    /// If this is used to prepare a peripheral-to-memory transfer, the caller
    /// must make sure that the buffer can be written to safely.
    ///
    /// # Panics
    ///
    /// Panics, if the length of the buffer is larger than `u16::max_value()`.
    ///
    /// Panics, if the buffer is not aligned to the word size.
    pub(crate) unsafe fn new<Word>(
        handle: &mut Handle,
        target: T,
        channel: C,
        buffer: Pin<B>,
        num_words: usize,
        address: u32,
        priority: Priority,
        dir: Direction,
        circular: bool,
    ) -> Self
    where
        B: Deref,
        B::Target: Buffer<Word>,
        Word: SupportedWordSize,
    {
        assert!(buffer.len() >= num_words);
        assert!(num_words <= u16::max_value() as usize);
        assert_eq!(buffer.as_ptr().align_offset(mem::size_of::<Word>()), 0);

        channel.select_target(handle, &target);
        channel.set_peripheral_address(handle, address);
        channel.set_memory_address(handle, buffer.as_ptr() as u32);
        channel.set_transfer_len(handle, num_words as u16);
        channel.configure::<Word>(handle, priority.0, dir.0, circular);

        Transfer {
            res: TransferResources {
                target,
                channel,
                buffer,
            },
            _state: Ready,
        }
    }

    /// Enables the provided interrupts
    ///
    /// This setting only affects this transfer. It doesn't affect transfer on
    /// other channels, or subsequent transfers on the same channel.
    pub fn enable_interrupts(&mut self, interrupts: Interrupts) {
        self.res.channel.enable_interrupts(interrupts);
    }

    /// Start the DMA transfer
    ///
    /// Consumes this instance of `Transfer` and returns a new one, with its
    /// state changed to indicate that the transfer has been started.
    pub fn start(self) -> Transfer<T, C, B, Started> {
        compiler_fence(Ordering::SeqCst);

        self.res.channel.start();

        Transfer {
            res: self.res,
            _state: Started,
        }
    }
}

impl<T, C, B> Transfer<T, C, B, Started>
where
    C: Channel,
{
    /// Indicates whether the transfer is still ongoing
    pub fn is_active(&self) -> bool {
        self.res.channel.is_active()
    }

    /// Waits for the transfer to finish and returns the owned resources
    ///
    /// This function will busily wait until the transfer is finished. If you
    /// don't want this, please call this function only once you know that the
    /// transfer has finished.
    ///
    /// This function will return immediately, if [`Transfer::is_active`]
    /// returns `false`.
    pub fn wait(self) -> TransferResourcesResult<T, C, B> {
        while self.res.channel.is_active() {
            if self.res.channel.error_occured() {
                return Err((self.res, Error));
            }
        }

        self.res.channel.clear_complete_flag();

        compiler_fence(Ordering::SeqCst);

        if self.res.channel.error_occured() {
            return Err((self.res, Error));
        }

        Ok(self.res)
    }

    /// Returns some transfer state
    ///
    /// The number of items to transfer, the half transfer flag, and the
    /// transfer completed flag.
    pub(crate) fn state(&self) -> (u16, bool, bool) {
        self.res.channel.transfer_state()
    }

    /// Clears the half transfer and transfer complete flags
    ///
    /// Be careful when calling this, as it can confuse the other methods. This
    /// method is designed to manage circular transfers only.
    pub(crate) fn clear_flags(&self) {
        self.res.channel.clear_flags()
    }
}

pub struct TransferResources<T, C, B> {
    pub target: T,
    pub channel: C,
    pub buffer: Pin<B>,
}

// Since `TransferResources` is used in the error variant of a `Result`, it
// needs to implement `Debug` for methods like `unwrap` to work. We can't just
// derive `Debug`, without requiring all type parameters to be `Debug`, which
// seems to restrictive.
impl<T, C, B> fmt::Debug for TransferResources<T, C, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TransferResources {{ ... }}")
    }
}

/// The priority of the DMA transfer
pub struct Priority(cr::PL_A);

impl Priority {
    pub fn low() -> Self {
        Self(cr::PL_A::Low)
    }

    pub fn medium() -> Self {
        Self(cr::PL_A::Medium)
    }

    pub fn high() -> Self {
        Self(cr::PL_A::High)
    }

    pub fn very_high() -> Self {
        Self(cr::PL_A::VeryHigh)
    }
}

/// The direction of the DMA transfer
pub(crate) struct Direction(cr::DIR_A);

impl Direction {
    pub fn memory_to_peripheral() -> Self {
        Self(cr::DIR_A::FromMemory)
    }

    pub fn peripheral_to_memory() -> Self {
        Self(cr::DIR_A::FromPeripheral)
    }
}

#[derive(Debug)]
pub struct Error;

pub trait Channel: Sized {
    fn select_target<T: Target<Self>>(&self, _: &mut Handle, target: &T);
    fn set_peripheral_address(&self, _: &mut Handle, address: u32);
    fn set_memory_address(&self, _: &mut Handle, address: u32);
    fn set_transfer_len(&self, _: &mut Handle, len: u16);
    fn configure<Word>(&self, _: &mut Handle, priority: cr::PL_A, dir: cr::DIR_A, circular: bool)
    where
        Word: SupportedWordSize;
    fn enable_interrupts(&self, interrupts: Interrupts);
    fn start(&self);
    fn is_active(&self) -> bool;
    fn clear_complete_flag(&self);
    fn error_occured(&self) -> bool;
    fn transfer_state(&self) -> (u16, bool, bool);
    fn clear_flags(&self);
}

macro_rules! impl_channel {
    (
        $(
            $channel:ident,
            $field:ident,
            $chfield:ident,
            $cxs:ident,
            $htif:ident,
            $tcif:ident,
            $teif:ident,
            $chtif:ident,
            $ctcif:ident,
            $cteif:ident;
        )*
    ) => {
        pub struct Channels {
            $(pub $field: $channel,)*
        }

        impl Default for Channels {
            fn default() -> Self {
                Self {
                    $($field: $channel(()),)*
                }
            }
        }

        impl Channels {
            pub fn new() -> Self {
                Default::default()
            }
        }

        $(
            pub struct $channel(());

            impl Channel for $channel {
                fn select_target<T: Target<Self>>(&self,
                    handle:  &mut Handle,
                    _target: &T,
                ) {
                    handle.dma.cselr.modify(|_, w| w.$cxs().bits(T::REQUEST));
                }

                fn set_peripheral_address(&self,
                    handle:  &mut Handle,
                    address: u32,
                ) {
                    // unsafe needed because of PAC. fine since pa takes all u32 values.
                    handle.dma.$chfield.par.write(|w| unsafe{w.pa().bits(address)});
                }

                fn set_memory_address(&self,
                    handle:  &mut Handle,
                    address: u32,
                ) {
                    // unsafe needed because of PAC. fine since ma takes all u32 values.
                    handle.dma.$chfield.mar.write(|w| unsafe{w.ma().bits(address)});
                }

                fn set_transfer_len(&self, handle: &mut Handle, len: u16) {
                    handle.dma.$chfield.ndtr.write(|w| w.ndt().bits(len));
                }

                fn configure<Word>(&self,
                    handle:   &mut Handle,
                    priority: cr::PL_A,
                    dir:      cr::DIR_A,
                    circular: bool,
                )
                    where Word: SupportedWordSize
                {
                    handle.dma.$chfield.cr.write(|w| {
                        w
                            // Word size in memory
                            .msize().variant(Word::size())
                            // Word size in peripheral
                            .psize().variant(Word::size())
                            // Memory-to-memory mode disabled
                            .mem2mem().disabled()
                            // Priority level
                            .pl().variant(priority)
                            // Increment memory pointer
                            .minc().enabled()
                            // Don't increment peripheral pointer
                            .pinc().disabled()
                            // Circular mode
                            .circ().bit(circular)
                            // Data transfer direction
                            .dir().variant(dir)
                            // Disable interrupts
                            .teie().disabled()
                            .htie().disabled()
                            .tcie().disabled()
                    });
                }

                fn enable_interrupts(&self, interrupts: Interrupts) {
                    // Safe, because we're only accessing a register that this
                    // channel has exclusive access to.
                    let ccr = &unsafe { &*pac::DMA1::ptr() }.$chfield.cr;

                    ccr.modify(|_, w|
                        w
                            .teie().bit(interrupts.transfer_error)
                            .htie().bit(interrupts.half_transfer)
                            .tcie().bit(interrupts.transfer_complete)
                    );
                }

                fn start(&self) {
                    // Safe, because we're only accessing a register that this
                    // channel has exclusive access to.
                    let ccr = &unsafe { &*pac::DMA1::ptr() }.$chfield.cr;

                    // Start transfer
                    ccr.modify(|_, w| w.en().enabled());
                }

                fn is_active(&self) -> bool {
                    // This is safe, for the following reasons:
                    // - We only do one atomic read of ISR.
                    let dma = unsafe { &*pac::DMA1::ptr() };
                    !dma.isr.read().$tcif().is_complete()
                }

                fn clear_complete_flag(&self) {
                    // This is safe, for the following reasons:
                    // - We only do one atomic read of ISR.
                    // - IFCR is a stateless register and we do one atomic
                    //   write.
                    // - This channel has exclusive access to CCRx.
                    let dma = unsafe { &*pac::DMA1::ptr() };

                    if dma.isr.read().$tcif().is_complete() {
                        dma.ifcr.write(|w| w.$ctcif().set_bit());
                        dma.$chfield.cr.modify(|_, w| w.en().disabled());
                    }
                }

                fn error_occured(&self) -> bool {
                    // This is safe, for the following reasons:
                    // - We only do one atomic read of ISR.
                    // - IFCR is a stateless register and we do one atomic
                    //   write.
                    let dma = unsafe { &*pac::DMA1::ptr() };

                    if dma.isr.read().$teif().is_error() {
                        dma.ifcr.write(|w| w.$cteif().set_bit());
                        true
                    }
                    else {
                        false
                    }
                }

                fn transfer_state(&self) -> (u16, bool, bool) {
                    // Safe, as we're only doing atomic reads.
                    let dma = unsafe { &*pac::DMA1::ptr() };

                    let isr = dma.isr.read();

                    let data_remaining = dma.$chfield.ndtr.read().ndt().bits();

                    let half_transfer     = isr.$htif().is_half();
                    let transfer_complete = isr.$tcif().is_complete();

                    (data_remaining, half_transfer, transfer_complete)
                }

                fn clear_flags(&self) {
                    // Safe, as we're only doing an atomic write to a stateless
                    // register.
                    let dma = unsafe { &*pac::DMA1::ptr() };

                    dma.ifcr.write(|w|
                        w
                            .$chtif().clear()
                            .$ctcif().clear()
                    );
                }
            }
        )*
    }
}

impl_channel!(
    Channel1, channel1, ch1,
        c1s, htif1, tcif1, teif1, chtif1, ctcif1, cteif1;
    Channel2, channel2, ch2,
        c2s, htif2, tcif2, teif2, chtif2, ctcif2, cteif2;
    Channel3, channel3, ch3,
        c3s, htif3, tcif3, teif3, chtif3, ctcif3, cteif3;
    Channel4, channel4, ch4,
        c4s, htif4, tcif4, teif4, chtif4, ctcif4, cteif4;
    Channel5, channel5, ch5,
        c5s, htif5, tcif5, teif5, chtif5, ctcif5, cteif5;
    Channel6, channel6, ch6,
        c6s, htif6, tcif6, teif6, chtif6, ctcif6, cteif6;
    Channel7, channel7, ch7,
        c7s, htif7, tcif7, teif7, chtif7, ctcif7, cteif7;
);

pub trait Target<Channel> {
    const REQUEST: u8;
}

macro_rules! impl_target {
    ($($target:ty, $channel:ty, $request:expr;)*) => {
        $(
            impl Target<$channel> for $target {
                const REQUEST: u8 = $request;
            }
        )*
    }
}

// See STM32L0x2 Reference Manual, table 51 (page 267).
impl_target!(
    // ADC
    adc::DmaToken, Channel1, 0;
    adc::DmaToken, Channel2, 0;
);

#[cfg(any(feature = "io-STM32L051", feature = "io-STM32L071"))]
impl_target!(
    // USART1
    serial::Tx<USART1>, Channel2, 3;
    serial::Tx<USART1>, Channel4, 3;
    serial::Rx<USART1>, Channel3, 3;
    serial::Rx<USART1>, Channel5, 3;
);

#[cfg(any(
    feature = "io-STM32L021",
    feature = "io-STM32L031",
    feature = "io-STM32L051",
    feature = "io-STM32L071",
))]
impl_target!(
    // USART2
    serial::Tx<USART2>, Channel4, 4;
    serial::Tx<USART2>, Channel7, 4;
    serial::Rx<USART2>, Channel5, 4;
    serial::Rx<USART2>, Channel6, 4;
);

#[cfg(feature = "stm32l0x2")]
#[cfg(any(
    feature = "io-STM32L021",
    feature = "io-STM32L031",
    feature = "io-STM32L051",
    feature = "io-STM32L071",
))]
impl_target!(
    // I2C1
    i2c::Tx<I2C1>, Channel2, 6;
    i2c::Rx<I2C1>, Channel3, 6;
    i2c::Tx<I2C1>, Channel6, 6;
    i2c::Rx<I2C1>, Channel7, 6;

    // I2C2
    i2c::Tx<I2C2>, Channel4, 7;
    i2c::Rx<I2C2>, Channel5, 7;

    // I2C3
    i2c::Tx<I2C3>, Channel2, 14;
    i2c::Rx<I2C3>, Channel3, 14;
    i2c::Tx<I2C3>, Channel4, 14;
    i2c::Rx<I2C3>, Channel5, 14;
);

// See STM32L0x2 Reference Manual, table 51 (page 267).
#[cfg(feature = "stm32l082")]
impl_target!(
    aes::Tx, Channel1, 11;
    aes::Tx, Channel5, 11;
    aes::Rx, Channel2, 11;
    aes::Rx, Channel3, 11;
);

impl_target!(
    // SPI1
    spi::Tx<SPI1>, Channel3, 1;
    spi::Rx<SPI1>, Channel2, 1;
);

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
impl_target!(
    // SPI2
    spi::Tx<SPI2>, Channel5, 2;
    spi::Rx<SPI2>, Channel4, 2;
    spi::Tx<SPI2>, Channel7, 2;
    spi::Rx<SPI2>, Channel6, 2;
);

/// Indicates that a DMA transfer is ready
pub struct Ready;

/// Indicates that a DMA transfer has been started
pub struct Started;

/// Implemented for types, that can be used as a buffer for DMA transfers
pub(crate) trait Buffer<Word> {
    fn as_ptr(&self) -> *const Word;
    fn len(&self) -> usize;
}

impl<T, Word> Buffer<Word> for T
where
    T: ?Sized + AsSlice<Element = Word>,
{
    fn as_ptr(&self) -> *const Word {
        self.as_slice().as_ptr()
    }

    fn len(&self) -> usize {
        self.as_slice().len()
    }
}

/// Can be used as a fallback [`Buffer`], if safer implementations can't be used
pub(crate) struct PtrBuffer<Word> {
    pub ptr: *const Word,
    pub len: usize,
}

// Required to make in possible to put this in a `Pin`, in a way that satisfies
// the requirements on `Transfer::new`.
impl<Word> Deref for PtrBuffer<Word> {
    type Target = Self;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl<Word> Buffer<Word> for PtrBuffer<Word> {
    fn as_ptr(&self) -> *const Word {
        self.ptr
    }

    fn len(&self) -> usize {
        self.len
    }
}

pub trait SupportedWordSize {
    fn size() -> cr::MSIZE_A;
}

impl SupportedWordSize for u8 {
    fn size() -> cr::MSIZE_A {
        cr::MSIZE_A::Bits8
    }
}

impl SupportedWordSize for u16 {
    fn size() -> cr::MSIZE_A {
        cr::MSIZE_A::Bits16
    }
}

impl SupportedWordSize for u32 {
    fn size() -> cr::MSIZE_A {
        cr::MSIZE_A::Bits32
    }
}

#[derive(Clone, Copy, Default)]
pub struct Interrupts {
    pub transfer_error: bool,
    pub half_transfer: bool,
    pub transfer_complete: bool,
}
