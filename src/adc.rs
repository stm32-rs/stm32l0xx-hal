//! # Analog to Digital converter

use core::{
    ops::DerefMut,
    pin::Pin,
    sync::atomic::{compiler_fence, Ordering},
};

use as_slice::AsMutSlice;

use crate::{
    gpio::*,
    hal::adc::{Channel, OneShot},
    pac::ADC,
    rcc::{Enable, Rcc},
};

use crate::dma::{self, Buffer as _};

pub trait AdcExt {
    fn constrain(self, rcc: &mut Rcc) -> Adc<Ready>;
}

impl AdcExt for ADC {
    fn constrain(self, rcc: &mut Rcc) -> Adc<Ready> {
        Adc::new(self, rcc)
    }
}

/// ADC Result Alignment
#[derive(PartialEq)]
pub enum Align {
    /// Right aligned results (least significant bits)
    ///
    /// Results in all precisions returning values from 0-(2^bits-1) in
    /// steps of 1.
    Right,
    /// Left aligned results (most significant bits)
    ///
    /// Results in all precisions returning a value in the range 0-65535.
    /// Depending on the precision the result will step by larger or smaller
    /// amounts.
    Left,
}

/// ADC Sampling Precision
#[derive(Copy, Clone, PartialEq)]
pub enum Precision {
    /// 12 bit precision
    B_12 = 0b00,
    /// 10 bit precision
    B_10 = 0b01,
    /// 8 bit precision
    B_8 = 0b10,
    /// 6 bit precision
    B_6 = 0b11,
}

/// ADC Sampling time
#[derive(Copy, Clone, PartialEq)]
pub enum SampleTime {
    /// 1.5 ADC clock cycles
    T_1_5 = 0b000,

    /// 3.5 ADC clock cycles
    T_3_5 = 0b001,

    /// 7.5 ADC clock cycles
    T_7_5 = 0b010,

    /// 12.5 ADC clock cycles
    T_12_5 = 0b011,

    /// 19.5 ADC clock cycles
    T_19_5 = 0b100,

    /// 39.5 ADC clock cycles
    T_39_5 = 0b101,

    /// 79.5 ADC clock cycles
    T_79_5 = 0b110,

    /// 160.5 ADC clock cycles
    T_160_5 = 0b111,
}

/// Analog to Digital converter interface
pub struct Adc<State> {
    rb: ADC,
    sample_time: SampleTime,
    align: Align,
    precision: Precision,
    _state: State,
}

impl Adc<Ready> {
    pub fn new(adc: ADC, rcc: &mut Rcc) -> Self {
        // Enable ADC clocks
        ADC::enable(rcc);
        adc.cr.modify(|_, w| w.advregen().set_bit());

        Self {
            rb: adc,
            sample_time: SampleTime::T_1_5,
            align: Align::Right,
            precision: Precision::B_12,
            _state: Ready,
        }
    }

    /// Set the Adc sampling time
    pub fn set_sample_time(&mut self, t_samp: SampleTime) {
        self.sample_time = t_samp;
    }

    /// Set the Adc result alignment
    pub fn set_align(&mut self, align: Align) {
        self.align = align;
    }

    /// Set the Adc precision
    pub fn set_precision(&mut self, precision: Precision) {
        self.precision = precision;
    }

    /// Starts a continuous conversion process
    ///
    /// The `channel` argument specifies which channel should be converted.
    ///
    /// The `trigger` argument specifies the trigger that will start each
    /// conversion sequence. This only configures the ADC peripheral to accept
    /// this trigger. The trigger itself must also be configured using its own
    /// peripheral API.
    ///
    /// In addition to the preceeding arguments that configure the ADC,
    /// additional arguments are required to configure the DMA transfer that is
    /// used to read the results from the ADC:
    /// - `dma` is a handle to the DMA peripheral.
    /// - `dma_chan` is the DMA channel used for the transfer. It needs to be
    ///   one of the channels that supports the ADC peripheral.
    /// - `buffer` is the buffer used to buffer the conversion results.
    ///
    /// # Panics
    ///
    /// Panics, if `buffer` is larger than 65535.
    pub fn start<DmaChan, Buf>(
        mut self,
        channels: impl Into<Channels>,
        trigger: Option<Trigger>,
        dma: &mut dma::Handle,
        dma_chan: DmaChan,
        buffer: Pin<Buf>,
    ) -> Adc<Active<DmaChan, Buf>>
    where
        DmaToken: dma::Target<DmaChan>,
        Buf: DerefMut + 'static,
        Buf::Target: AsMutSlice<Element = u16>,
        DmaChan: dma::Channel,
    {
        // The ADC can support only one DMA transfer at a time, so only one of
        // these DMA tokens must exist at a time. We guarantee this by only
        // creating it in this method, that can only be called in the ADC's
        // `Ready` state. The DMA transfer is ended and the token associated
        // with it is dropped before we return to the `Ready` state.
        let dma_token = DmaToken(());

        let num_words = (*buffer).len();

        // Safe, because we're only taking the address of a register.
        let address = &self.rb.dr as *const _ as u32;

        // The cast to `u16` could truncate the value, but if it does,
        // `Transfer::new` is going to panic anyway.
        let buffer_unsafe = Buffer {
            ptr: buffer.as_ptr(),
            len: buffer.len() as u16,
            pos: 0,
            dma_pos: 0,

            r_gt_w: false,
        };

        // Safe, because the trait bounds of this method guarantee that the
        // buffer can be written to.
        let transfer = unsafe {
            dma::Transfer::new(
                dma,
                dma_token,
                dma_chan,
                buffer,
                num_words,
                address,
                dma::Priority::high(),
                dma::Direction::peripheral_to_memory(),
                true,
            )
        }
        .start();

        let continous = trigger.is_none();

        self.power_up();
        self.configure(channels, continous, trigger);

        Adc {
            rb: self.rb,
            sample_time: self.sample_time,
            align: self.align,
            precision: self.precision,
            _state: Active {
                buffer: buffer_unsafe,
                transfer,
            },
        }
    }
}

impl<DmaChan, Buffer> Adc<Active<DmaChan, Buffer>>
where
    DmaChan: dma::Channel,
{
    /// Returns an iterator over all currently available values
    ///
    /// The iterator iterates over all buffered values. It returns `None`, once
    /// the end of the buffer has been reached.
    pub fn read_available(
        &mut self,
    ) -> Result<impl Iterator<Item = Result<u16, Error>> + '_, Error> {
        if self.rb.isr.read().ovr().is_overrun() {
            self.rb.isr.write(|w| w.ovr().clear());
            return Err(Error::AdcOverrun);
        }

        Ok(ReadAvailable {
            buffer: &mut self._state.buffer,
            transfer: &mut self._state.transfer,
        })
    }
}

impl<State> Adc<State> {
    pub fn release(self) -> ADC {
        self.rb
    }

    fn power_up(&mut self) {
        self.rb.isr.modify(|_, w| w.adrdy().set_bit());
        self.rb.cr.modify(|_, w| w.aden().set_bit());
        while self.rb.isr.read().adrdy().bit_is_clear() {}
    }

    fn power_down(&mut self) {
        self.rb.cr.modify(|_, w| w.addis().set_bit());
        self.rb.isr.modify(|_, w| w.adrdy().set_bit());
        while self.rb.cr.read().aden().bit_is_set() {}
    }

    fn configure(&mut self, channels: impl Into<Channels>, cont: bool, trigger: Option<Trigger>) {
        self.rb.cfgr1.write(|w| {
            w.res().bits(self.precision as u8);
            w.cont().bit(cont);
            w.align().bit(self.align == Align::Left);
            // DMA circular mode
            w.dmacfg().set_bit();
            // Generate DMA requests
            w.dmaen().set_bit();

            if let Some(trigger) = trigger {
                // Select hardware trigger
                w.extsel().bits(trigger as u8);
                // Enable hardware trigger on rising edge
                w.exten().rising_edge();
            }

            w
        });

        self.rb
            .smpr
            .modify(|_, w| w.smp().bits(self.sample_time as u8));

        self.rb.chselr.write(|w|
            // Safe, as long as there are no `Channel` implementations that
            // define invalid values.
            unsafe { w.bits(channels.into().flags) });

        self.rb.isr.modify(|_, w| w.eos().set_bit());
        self.rb.cr.modify(|_, w| w.adstart().set_bit());
    }
}

impl<WORD, PIN> OneShot<Adc<Ready>, WORD, PIN> for Adc<Ready>
where
    WORD: From<u16>,
    PIN: Channel<Adc<Ready>, ID = u8>,
{
    type Error = ();

    fn read(&mut self, _: &mut PIN) -> nb::Result<WORD, Self::Error> {
        self.power_up();
        self.configure(
            Channels {
                flags: 0x1 << PIN::channel(),
            },
            false,
            None,
        );

        while self.rb.isr.read().eos().bit_is_clear() {}

        let res = self.rb.dr.read().bits() as u16;
        let val = if self.align == Align::Left && self.precision == Precision::B_6 {
            res << 8
        } else {
            res
        };

        self.power_down();
        Ok(val.into())
    }
}

/// Indicates that the ADC peripheral is ready
pub struct Ready;

/// Indicates that the ADC peripheral is performing conversions
pub struct Active<DmaChan, Buf> {
    transfer: dma::Transfer<DmaToken, DmaChan, Buf, dma::Started>,
    buffer: Buffer,
}

/// A collection of channels
///
/// Used to set up multi-channel conversions.
#[derive(Default)]
pub struct Channels {
    flags: u32,
}

impl Channels {
    pub fn new() -> Channels {
        Default::default()
    }

    /// Adds a channel to the collection
    pub fn add<C>(&mut self, _: C)
    where
        C: Channel<Adc<Ready>, ID = u8>,
    {
        self.flags |= 0x1 << C::channel()
    }
}

impl<C> From<C> for Channels
where
    C: Channel<Adc<Ready>, ID = u8>,
{
    fn from(channel: C) -> Self {
        let mut c = Channels { flags: 0 };
        c.add(channel);
        c
    }
}

/// Hardware triggers that can start an ADC conversion
#[repr(u8)]
pub enum Trigger {
    /// TRG0
    TIM6_TRGO = 0b000,

    /// TRG1
    TIM21_CH2 = 0b001,

    /// TRG2
    TIM2_TRGO = 0b010,

    /// TRG3
    TIM2_CH4 = 0b011,

    /// TRG4
    TIM22_TRGO = 0b100,

    /// TRG5
    ///
    /// Only available on Category 5 devices.
    #[cfg(any(feature = "stm32l072", feature = "stm32l082"))]
    TIM2_CH3 = 0b101,

    /// TRG6
    TIM3_TRGO = 0b110,

    /// TRG7
    EXTI11 = 0b111,
}

/// Provides access to the buffer that the DMA writes ADC values into
///
/// Since the DMA transfer takes ownership of the buffer, we need to access it
/// with unsafe means. This struct is a safe wrapper around this unsafe access.
struct Buffer {
    ptr: *const u16,
    len: u16,
    pos: u16,
    dma_pos: u16,

    /// Indicates order of read and write indices
    ///
    /// This is initially `false`, indicating that the read position (the `pos`
    /// field) is smaller than or equal to the write position (internally
    /// managed by the DMA peripheral).
    ///
    /// Once the write position wraps around the buffer boundary, this becomes
    /// `true` until the read position also wraps around.
    r_gt_w: bool,
}

impl Buffer {
    fn read<T, C, B>(
        &mut self,
        transfer: &dma::Transfer<T, C, B, dma::Started>,
    ) -> Option<Result<u16, Error>>
    where
        C: dma::Channel,
    {
        let transfer_state = self.transfer_state(transfer);
        if self.check_overrun(transfer_state) {
            return Some(Err(Error::BufferOverrun));
        }

        if self.pos == transfer_state.pos {
            // No overrun detected, but read and write positions are equal. This
            // can only mean that the buffer is empty.
            return None;
        }

        // Safe, as we know that `ptr` and `len` define a valid buffer, and we
        // make sure that `pos <= len`. There's a race condition between this
        // line and the DMA peripheral, of course, but we take care of that with
        // these overrun checks.
        //
        // The cast is fine too. This is a 32-bit platform, so casting a `u16`
        // to an `isize` will never truncate the value.
        compiler_fence(Ordering::SeqCst);
        let value = unsafe { *self.ptr.offset(self.pos as isize) };
        compiler_fence(Ordering::SeqCst);

        // At this point we know that there was no overrun before we started
        // reading, but of course the DMA might have overtaken us since that
        // check. Let's check again. If there's still no overrun, we know that
        // our value is valid.
        let transfer_state = self.transfer_state(transfer);
        if self.check_overrun(transfer_state) {
            // Strictly speaking, the overrun might have happened after our
            // read, and `value` might be valid. No way to know for sure though,
            // so let's assume overrun.
            return Some(Err(Error::BufferOverrun));
        }

        // Now we know that the value we read is totally fine. Let's advance the
        // read position to finish up here.
        self.pos = self.pos.wrapping_add(1);
        if self.pos == 0 || self.pos >= self.len {
            // We advanced beyond the end of the buffer, which means we need to
            // wrap around to the beginning.
            self.pos = 0;
            self.r_gt_w = false;
        }

        Some(Ok(value))
    }

    fn transfer_state<T, C, B>(
        &self,
        transfer: &dma::Transfer<T, C, B, dma::Started>,
    ) -> TransferState
    where
        C: dma::Channel,
    {
        let (remaining, half, complete) = transfer.state();
        transfer.clear_flags();

        // Let's translate what we got from the DMA peripheral into a write
        // position that we can compare with our read position.
        let pos = self.len - remaining;

        TransferState {
            pos,
            half,
            complete,
        }
    }

    fn check_overrun(&mut self, transfer_state: TransferState) -> bool {
        let overrun = self.check_overrun_inner(transfer_state);
        self.dma_pos = transfer_state.pos; // Update our state of the DMA

        if overrun {
            // An overrun occured, but that is not a catastrophic error. Values
            // got lost, but that doesn't mean we can't read the new values
            // starting now. Let's get the buffer into a consistent state to
            // make that possible.
            //
            // There are various ways to go about this. What we're doing here is
            // to throw away all values in the buffer and start again with an
            // empty buffer, because that minimizes the likelihood of getting
            // another overrun right away.
            //
            // Maybe doing the opposite, setting the read position so that the
            // buffer is full, to minimize lost values, would be better. But
            // then we should give the user the option to empty the buffer
            // manually. I've chosen to go with the simpler option for now.
            self.pos = transfer_state.pos;
            self.r_gt_w = false;
        }

        overrun
    }

    fn check_overrun_inner(&mut self, transfer_state: TransferState) -> bool {
        if transfer_state.half && transfer_state.complete {
            // Each time we attempt a read, we clear both flags. If both flags
            // are set, then basically anything could have happened in between,
            // so we have to assume an overrun.
            //
            // Please note that it's possible that the DMA has written beyond
            // the half point and wrapped around, causing both of the flags to
            // be set, without passing our current reading position. However,
            // there's no way to distinguish this case from the DMA having
            // passed those marks multiple times, so we have to be conservative
            // and assume an overrun.
            return true;
        }

        if transfer_state.complete && self.dma_pos < transfer_state.pos {
            // If the complete flag is set and our previous position is less than
            // the current position then an overrun must have occurred
            // This is because the DMA must have wrapped to 0 and then ran past us again
            return true;
        }

        // Don't use the transfer complete flag to detect wrap (aside from the overrun above)
        // There is a timing issue with reading and clearing it so depend on relative positions
        if transfer_state.pos < self.dma_pos {
            // The write has wrapped beyond the buffer boundary and started
            // again at the beginning of the buffer. This is completely normal,
            // but it affects how we detect an overrun.

            if self.r_gt_w {
                // The read position was greater than the write position, so if
                // the write position wrapped, it must have overtaken the read
                // position. This is an overrun.
                return true;
            }

            // The write position has wrapped, so now the read position needs
            // to be greater than the write position.
            self.r_gt_w = true;
        }

        // At this point we know that everything _could_ be alright, judging
        // from the combination of flags we checked so far. We still need to
        // compare read and write positions to make sure that we don't actually
        // have an overrun.
        if self.r_gt_w {
            self.pos <= transfer_state.pos
        } else {
            self.pos > transfer_state.pos
        }
    }
}

/// Internal struct to represent the current state of the DMA transfer
#[derive(Clone, Copy, Debug)]
struct TransferState {
    pos: u16,
    half: bool,
    complete: bool,
}

/// Iterator over buffered ADC values
pub struct ReadAvailable<'r, T, C, B> {
    buffer: &'r mut Buffer,
    transfer: &'r dma::Transfer<T, C, B, dma::Started>,
}

impl<T, C, B> Iterator for ReadAvailable<'_, T, C, B>
where
    C: dma::Channel,
{
    type Item = Result<u16, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.read(self.transfer)
    }
}

/// Used for DMA transfers
///
/// This is an internal implementation detail. It is only public because it
/// leaks out of a public API in the form of a `where` clause.
pub struct DmaToken(());

/// Represents an ADC error
#[derive(Debug)]
pub enum Error {
    /// Indicates that converted data was not read in time
    ///
    /// This happens if either the user or the DMA (depending on mode) did not
    /// read the converted value before another one was ready.
    AdcOverrun,

    /// Indicates that values in the internal buffer have been overwritten
    ///
    /// This is not a critical error, as a circular buffer is used, and the DMA
    /// just keeps writing more values. It does mean that some values in the
    /// buffer were overwritten though.
    BufferOverrun,
}

macro_rules! int_adc {
    ($($Chan:ident: ($chan:expr, $en:ident)),+ $(,)*) => {
        $(
            pub struct $Chan;

            impl Default for $Chan {
                fn default() -> Self {
                    Self {}
                }
            }

            impl $Chan {
                pub fn new() -> Self {
                    Default::default()
                }

                pub fn enable(&mut self, adc: &mut Adc<Ready>) {
                    adc.rb.ccr.modify(|_, w| w.$en().set_bit());
                }

                pub fn disable(&mut self, adc: &mut Adc<Ready>) {
                    adc.rb.ccr.modify(|_, w| w.$en().clear_bit());
                }
            }

            impl Channel<Adc<Ready>> for $Chan {
                type ID = u8;

                fn channel() -> u8 {
                    $chan
                }
            }
        )+
    };
}

macro_rules! adc_pins {
    ($($Chan:ty: ($pin:ty, $chan:expr)),+ $(,)*) => {
        $(
            impl Channel<Adc<Ready>> for $pin {
                type ID = u8;

                fn channel() -> u8 { $chan }
            }
        )+
    };
}

int_adc! {
    VTemp: (18, tsen),
    VRef: (17, vrefen),
}

adc_pins! {
    Channel0: (gpioa::PA0<Analog>, 0u8),
    Channel1: (gpioa::PA1<Analog>, 1u8),
    Channel2: (gpioa::PA2<Analog>, 2u8),
    Channel3: (gpioa::PA3<Analog>, 3u8),
    Channel4: (gpioa::PA4<Analog>, 4u8),
    Channel5: (gpioa::PA5<Analog>, 5u8),
    Channel6: (gpioa::PA6<Analog>, 6u8),
    Channel7: (gpioa::PA7<Analog>, 7u8),
    Channel8: (gpiob::PB0<Analog>, 8u8),
    Channel9: (gpiob::PB1<Analog>, 9u8),
}

#[cfg(all(feature = "stm32l052", any(feature = "lqfp64", feature = "tfbga64",),))]
adc_pins! {
    Channel10: (gpioc::PC0<Analog>, 10u8),
    Channel11: (gpioc::PC1<Analog>, 11u8),
    Channel12: (gpioc::PC2<Analog>, 12u8),
}

#[cfg(all(
    feature = "stm32l072",
    any(
        feature = "lqfp64",
        feature = "lqfp100",
        feature = "tfbga64",
        feature = "ufbga64",
        feature = "ufbga100",
        feature = "wlcsp49",
    ),
))]
adc_pins! {
    Channel10: (gpioc::PC0<Analog>, 10u8),
    Channel11: (gpioc::PC1<Analog>, 11u8),
    Channel12: (gpioc::PC2<Analog>, 12u8),
}

#[cfg(all(feature = "stm32l082", feature = "wlcsp49"))]
adc_pins! {
    Channel10: (gpioc::PC0<Analog>, 10u8),
    Channel11: (gpioc::PC1<Analog>, 11u8),
    Channel12: (gpioc::PC2<Analog>, 12u8),
}

#[cfg(all(feature = "stm32l052", feature = "lqfp64"))]
adc_pins! {
    Channel13: (gpioc::PC3<Analog>, 13u8),
}

#[cfg(all(
    feature = "stm32l072",
    any(feature = "lqfp64", feature = "lqfp100", feature = "ufbga100",),
))]
adc_pins! {
    Channel13: (gpioc::PC3<Analog>, 13u8),
}

#[cfg(all(feature = "stm32l052", any(feature = "lqfp64", feature = "tfbga64",),))]
adc_pins! {
    Channel14: (gpioc::PC4<Analog>, 14u8),
    Channel15: (gpioc::PC5<Analog>, 15u8),
}

#[cfg(all(
    feature = "stm32l072",
    any(
        feature = "lqfp64",
        feature = "lqfp100",
        feature = "tfbga64",
        feature = "ufbga64",
        feature = "ufbga100",
    ),
))]
adc_pins! {
    Channel14: (gpioc::PC4<Analog>, 14u8),
    Channel15: (gpioc::PC5<Analog>, 15u8),
}
