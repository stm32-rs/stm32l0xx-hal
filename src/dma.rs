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
#![cfg_attr(not(feature = "stm32l082"), allow(unused, unused_macros))]


use core::{
    fmt,
    mem,
    ops::Deref,
    pin::Pin,
    sync::atomic::{
        compiler_fence,
        Ordering,
    }
};

use as_slice::AsSlice;

use crate::{
    pac::{
        self,
        dma1::ccr1,
        USART1,
        USART2,
    },
    rcc::Rcc,
    serial,
};

#[cfg(feature = "stm32l082")]
use crate::aes;


/// Entry point to the DMA API
pub struct DMA {
    /// Handle to the DMA peripheral
    pub handle: Handle,

    /// DMA channels
    pub channels: Channels
}

impl DMA {
    /// Create an instance of the DMA API
    pub fn new(dma: pac::DMA1, rcc: &mut Rcc) -> Self {
        // Reset peripheral
        rcc.rb.ahbrstr.modify(|_, w| w.dmarst().set_bit());
        rcc.rb.ahbrstr.modify(|_, w| w.dmarst().clear_bit());

        // Enable peripheral clock
        rcc.rb.ahbenr.modify(|_, w| w.dmaen().set_bit());

        Self {
            handle:   Handle { dma },
            channels: Channels::new(),
        }
    }
}


/// Handle to the DMA peripheral
pub struct Handle {
    dma: pac::DMA1,
}


pub struct Transfer<T, C, B, State> {
    res:    TransferResources<T, C, B>,
    _state: State,
}

impl<T, C, B> Transfer<T, C, B, Ready>
    where
        T: Target<C>,
        C: Channel,
{
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
        handle:   &mut Handle,
        target:   T,
        channel:  C,
        buffer:   Pin<B>,
        address:  u32,
        priority: Priority,
        dir:      Direction,
    )
        -> Self
        where
            B:         Deref,
            B::Target: Buffer<Word>,
            Word:      SupportedWordSize,
    {
        assert!(buffer.len() <= u16::max_value() as usize);
        assert_eq!(buffer.as_ptr().align_offset(mem::size_of::<Word>()), 0);

        channel.select_target(handle, &target);
        channel.set_peripheral_address(handle, address);
        channel.set_memory_address(handle, buffer.as_ptr() as u32);
        channel.set_transfer_len(handle, buffer.len() as u16);
        channel.configure::<Word>(handle, priority.0, dir.0);

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
    /// state changes to indicate that the transfer has been started.
    pub fn start(self) -> Transfer<T, C, B, Started> {
        compiler_fence(Ordering::SeqCst);

        self.res.channel.start();

        Transfer {
            res:    self.res,
            _state: Started,
        }
    }
}

impl<T, C, B> Transfer<T, C, B, Started>
    where C: Channel
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
    pub fn wait(self)
        -> Result<
            TransferResources<T, C, B>,
            (TransferResources<T, C, B>, Error)
        >
    {
        while self.is_active() {
            if self.res.channel.error_occured() {
                return Err((self.res, Error));
            }
        }

        compiler_fence(Ordering::SeqCst);

        if self.res.channel.error_occured() {
            return Err((self.res, Error));
        }

        Ok(self.res)
    }
}


pub struct TransferResources<T, C, B> {
    pub target:  T,
    pub channel: C,
    pub buffer:  Pin<B>,
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
pub struct Priority(ccr1::PLW);

impl Priority {
    pub fn low() -> Self {
        Self(ccr1::PLW::LOW)
    }

    pub fn medium() -> Self {
        Self(ccr1::PLW::MEDIUM)
    }

    pub fn high() -> Self {
        Self(ccr1::PLW::HIGH)
    }

    pub fn very_high() -> Self {
        Self(ccr1::PLW::VERYHIGH)
    }
}


/// The direction of the DMA transfer
pub(crate) struct Direction(ccr1::DIRW);

impl Direction {
    pub fn memory_to_peripheral() -> Self {
        Self(ccr1::DIRW::FROMMEMORY)
    }

    pub fn peripheral_to_memory() -> Self {
        Self(ccr1::DIRW::FROMPERIPHERAL)
    }
}


#[derive(Debug)]
pub struct Error;


pub trait Channel: Sized {
    fn select_target<T: Target<Self>>(&self, _: &mut Handle, target: &T);
    fn set_peripheral_address(&self, _: &mut Handle, address: u32);
    fn set_memory_address(&self, _: &mut Handle, address: u32);
    fn set_transfer_len(&self, _: &mut Handle, len: u16);
    fn configure<Word>(&self,
        _:        &mut Handle,
        priority: ccr1::PLW,
        dir:      ccr1::DIRW,
    )
        where Word: SupportedWordSize;
    fn enable_interrupts(&self, interrupts: Interrupts);
    fn start(&self);
    fn is_active(&self) -> bool;
    fn error_occured(&self) -> bool;
}

macro_rules! impl_channel {
    (
        $(
            $channel:ident,
            $field:ident,
            $cxs:ident,
            $cpar:ident,
            $cmar:ident,
            $cndtr:ident,
            $ccr:ident,
            $tcif:ident,
            $teif:ident,
            $ctcif:ident,
            $cteif:ident;
        )*
    ) => {
        pub struct Channels {
            $(pub $field: $channel,)*
        }

        impl Channels {
            pub fn new() -> Self {
                Self {
                    $($field: $channel(()),)*
                }
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
                    handle.dma.$cpar.write(|w| w.pa().bits(address));
                }

                fn set_memory_address(&self,
                    handle:  &mut Handle,
                    address: u32,
                ) {
                    handle.dma.$cmar.write(|w| w.ma().bits(address));
                }

                fn set_transfer_len(&self, handle: &mut Handle, len: u16) {
                    handle.dma.$cndtr.write(|w| w.ndt().bits(len));
                }

                fn configure<Word>(&self,
                    handle:   &mut Handle,
                    priority: ccr1::PLW,
                    dir:      ccr1::DIRW,
                )
                    where Word: SupportedWordSize
                {
                    handle.dma.$ccr.write(|w| {
                        // Safe, as the enum we use should only provide valid
                        // bit patterns.
                        let w = unsafe {
                            w
                                // Word size in memory
                                .msize().bits(Word::size()._bits())
                                // Word size in peripheral
                                .psize().bits(Word::size()._bits())
                        };

                        w
                            // Memory-to-memory mode disabled
                            .mem2mem().disabled()
                            // Priority level
                            .pl().bits(priority._bits())
                            // Increment memory pointer
                            .minc().enabled()
                            // Don't increment peripheral pointer
                            .pinc().disabled()
                            // Circular mode disabled
                            .circ().disabled()
                            // Data transfer direction
                            .dir().bit(dir._bits())
                            // Disable interrupts
                            .teie().disabled()
                            .htie().disabled()
                            .tcie().disabled()
                    });
                }

                fn enable_interrupts(&self, interrupts: Interrupts) {
                    // Safe, because we're only accessing a register that this
                    // channel has exclusive access to.
                    let ccr = &unsafe { &*pac::DMA1::ptr() }.$ccr;

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
                    let ccr = &unsafe { &*pac::DMA1::ptr() }.$ccr;

                    // Start transfer
                    ccr.modify(|_, w| w.en().enabled());
                }

                fn is_active(&self) -> bool {
                    // This is safe, for the following reasons:
                    // - We only do one atomic read of ISR.
                    // - IFCR is a stateless register and we don one atomic
                    //   write.
                    // - This channel has exclusive access to CCRx.
                    let dma = unsafe { &*pac::DMA1::ptr() };

                    if dma.isr.read().$tcif().is_complete() {
                        dma.ifcr.write(|w| w.$ctcif().set_bit());
                        dma.$ccr.modify(|_, w| w.en().disabled());
                        false
                    }
                    else {
                        true
                    }
                }

                fn error_occured(&self) -> bool {
                    // This is safe, for the following reasons:
                    // - We only do one atomic read of ISR.
                    // - IFCR is a stateless register and we don one atomic
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
            }
        )*
    }
}

impl_channel!(
    Channel1, channel1,
        c1s, cpar1, cmar1, cndtr1, ccr1,
        tcif1, teif1, ctcif1, cteif1;
    Channel2, channel2,
        c2s, cpar2, cmar2, cndtr2, ccr2,
        tcif2, teif2, ctcif2, cteif2;
    Channel3, channel3,
        c3s, cpar3, cmar3, cndtr3, ccr3,
        tcif3, teif3, ctcif3, cteif3;
    Channel4, channel4,
        c4s, cpar4, cmar4, cndtr4, ccr4,
        tcif4, teif4, ctcif4, cteif4;
    Channel5, channel5,
        c5s, cpar5, cmar5, cndtr5, ccr5,
        tcif5, teif5, ctcif5, cteif5;
    Channel6, channel6,
        c6s, cpar6, cmar6, cndtr6, ccr6,
        tcif6, teif6, ctcif6, cteif6;
    Channel7, channel7,
        c7s, cpar7, cmar7, cndtr7, ccr7,
        tcif7, teif7, ctcif7, cteif7;
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

impl_target!(
    serial::Tx<USART1>, Channel2, 3;
    serial::Tx<USART1>, Channel4, 3;
    serial::Rx<USART1>, Channel3, 3;
    serial::Rx<USART1>, Channel5, 3;

    serial::Tx<USART2>, Channel4, 4;
    serial::Tx<USART2>, Channel7, 4;
    serial::Rx<USART2>, Channel5, 4;
    serial::Rx<USART2>, Channel6, 4;
);

// See STM32L0x2 Reference Manual, table 51 (page 267).
#[cfg(feature = "stm32l082")]
impl_target!(
    aes::Tx, Channel1, 11;
    aes::Tx, Channel5, 11;
    aes::Rx, Channel2, 11;
    aes::Rx, Channel3, 11;
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
    where T: ?Sized + AsSlice<Element=Word>
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
    fn size() -> ccr1::MSIZEW;
}

impl SupportedWordSize for u8 {
    fn size() -> ccr1::MSIZEW {
        ccr1::MSIZEW::BITS8
    }
}

impl SupportedWordSize for u16 {
    fn size() -> ccr1::MSIZEW {
        ccr1::MSIZEW::BITS16
    }
}

impl SupportedWordSize for u32 {
    fn size() -> ccr1::MSIZEW {
        ccr1::MSIZEW::BITS32
    }
}


#[derive(Clone, Copy)]
pub struct Interrupts {
    pub transfer_error:    bool,
    pub half_transfer:     bool,
    pub transfer_complete: bool,
}

impl Default for Interrupts {
    fn default() -> Self {
        Self {
            transfer_error:    false,
            half_transfer:     false,
            transfer_complete: false,
        }
    }
}
