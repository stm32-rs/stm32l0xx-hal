use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr,
};

use as_slice::{AsMutSlice, AsSlice};
use embedded_time::rate::Hertz;

use crate::dma::{self, Buffer};

use crate::gpio::gpioa::*;
use crate::gpio::gpiob::*;

use crate::gpio::{AltMode, Analog};
use crate::hal;
use crate::pac::SPI1;
#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
use crate::pac::SPI2;
use crate::rcc::{Enable, Rcc};

pub use hal::spi::{Mode, Phase, Polarity, MODE_0, MODE_1, MODE_2, MODE_3};

/// SPI error
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Busy,
    FrameError,
    /// Overrun occurred
    Overrun,
    /// Mode fault occurred
    ModeFault,
    /// CRC error
    Crc,
}

pub trait Pins<SPI> {
    fn setup(&self);
}
pub trait PinSck<SPI> {
    fn setup(&self);
}
pub trait PinMiso<SPI> {
    fn setup(&self);
}
pub trait PinMosi<SPI> {
    fn setup(&self);
}

impl<SPI, SCK, MISO, MOSI> Pins<SPI> for (SCK, MISO, MOSI)
where
    SCK: PinSck<SPI>,
    MISO: PinMiso<SPI>,
    MOSI: PinMosi<SPI>,
{
    fn setup(&self) {
        self.0.setup();
        self.1.setup();
        self.2.setup();
    }
}

/// A filler type for when the SCK pin is unnecessary
pub struct NoSck;
impl NoSck {
    fn set_alt_mode(&self, _some: Option<u32>) {}
}
/// A filler type for when the Miso pin is unnecessary
pub struct NoMiso;
impl NoMiso {
    fn set_alt_mode(&self, _some: Option<u32>) {}
}
/// A filler type for when the Mosi pin is unnecessary
pub struct NoMosi;
impl NoMosi {
    fn set_alt_mode(&self, _some: Option<u32>) {}
}

macro_rules! pins {
    ($($SPIX:ty:
        SCK: [$([$SCK:ty, $ALTMODESCK:path]),*]
        MISO: [$([$MISO:ty, $ALTMODEMISO:path]),*]
        MOSI: [$([$MOSI:ty, $ALTMODEMOSI:path]),*])+) => {
        $(
            $(
                impl PinSck<$SPIX> for $SCK {
                    fn setup(&self) {
                        self.set_alt_mode($ALTMODESCK);
                    }
                }
            )*
            $(
                impl PinMiso<$SPIX> for $MISO {
                    fn setup(&self) {
                        self.set_alt_mode($ALTMODEMISO);
                    }
                }
            )*
            $(
                impl PinMosi<$SPIX> for $MOSI {
                    fn setup(&self) {
                        self.set_alt_mode($ALTMODEMOSI);
                    }
                }
            )*
        )+
    }
}

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
pins! {
    SPI1:
        SCK: [
            [NoSck, None],
            [PB3<Analog>, AltMode::AF0],
            [PA5<Analog>, AltMode::AF0]
        ]
        MISO: [
            [NoMiso, None],
            [PA6<Analog>, AltMode::AF0],
            [PA11<Analog>, AltMode::AF0],
            [PB4<Analog>, AltMode::AF0]
        ]
        MOSI: [
            [NoMosi, None],
            [PA7<Analog>, AltMode::AF0],
            [PA12<Analog>, AltMode::AF0],
            [PB5<Analog>, AltMode::AF0]
        ]
}

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
pins! {
    SPI2:
        SCK: [
            [NoSck, None],
            [PB13<Analog>, AltMode::AF0]
        ]
        MISO: [
            [NoMiso, None],
            [PB14<Analog>, AltMode::AF0]
        ]
        MOSI: [
            [NoMosi, None],
            [PB15<Analog>, AltMode::AF0]
        ]
}

#[cfg(feature = "stm32l0x1")]
pins! {
    SPI1:
        SCK: [
            [NoSck, None],
            [PA5<Analog>, AltMode::AF0],
            [PB3<Analog>, AltMode::AF0]
        ]
        MISO: [
            [NoMiso, None],
            [PA6<Analog>, AltMode::AF0],
            [PA11<Analog>, AltMode::AF0],
            [PB4<Analog>, AltMode::AF0]
        ]
        MOSI: [
            [NoMosi, None],
            [PA7<Analog>, AltMode::AF0],
            [PA12<Analog>, AltMode::AF0],
            [PB5<Analog>, AltMode::AF0]
        ]
}

#[derive(Debug)]
pub struct Spi<SPI, PINS> {
    spi: SPI,
    pins: PINS,
}

pub trait SpiExt<SPI>: Sized {
    fn spi<PINS, T>(self, pins: PINS, mode: Mode, freq: T, rcc: &mut Rcc) -> Spi<SPI, PINS>
    where
        PINS: Pins<SPI>,
        T: Into<Hertz>;
}

macro_rules! spi {
    ($($SPIX:ident: ($spiX:ident, $pclkX:ident),)+) => {
        $(
            impl<PINS> Spi<$SPIX, PINS> {
                pub fn $spiX<T>(
                    spi: $SPIX,
                    pins: PINS,
                    mode: Mode,
                    freq: T,
                    rcc: &mut Rcc
                ) -> Self
                where
                PINS: Pins<$SPIX>,
                T: Into<Hertz>
                {
                    pins.setup();

                    // Enable clock for SPI
                    <$SPIX>::enable(rcc);

                    spi.cr2.write(|w| {
                        // disable SS output
                        w.ssoe().clear_bit();
                        // enable DMA reception
                        w.rxdmaen().set_bit();
                        // enable DMA transmission
                        w.txdmaen().set_bit()
                    });

                    let spi_freq = freq.into().0;
                    let apb_freq = rcc.clocks.$pclkX().0;
                    let br = match apb_freq / spi_freq {
                        0 => unreachable!(),
                        1..=2 => 0b000,
                        3..=5 => 0b001,
                        6..=11 => 0b010,
                        12..=23 => 0b011,
                        24..=47 => 0b100,
                        48..=95 => 0b101,
                        96..=191 => 0b110,
                        _ => 0b111,
                    };

                    // mstr: master configuration
                    // lsbfirst: MSB first
                    // ssm: enable software slave management (NSS pin free for other uses)
                    // ssi: set nss high = master mode
                    // dff: 8 bit frames
                    // bidimode: 2-line unidirectional
                    // spe: enable the SPI bus
                    #[allow(unused)]
                    spi.cr1.write(|w| unsafe {
                        w.cpha()
                            .bit(mode.phase == Phase::CaptureOnSecondTransition)
                            .cpol()
                            .bit(mode.polarity == Polarity::IdleHigh)
                            .mstr()
                            .set_bit()
                            .br()
                            .bits(br)
                            .lsbfirst()
                            .clear_bit()
                            .ssm()
                            .set_bit()
                            .ssi()
                            .set_bit()
                            .rxonly()
                            .clear_bit()
                            .dff()
                            .clear_bit()
                            .bidimode()
                            .clear_bit()
                            .spe()
                            .set_bit()
                    });

                    Spi { spi, pins }
                }

                pub fn free(self) -> ($SPIX, PINS) {
                    (self.spi, self.pins)
                }

                pub fn read_all<Channel, Buffer>(
                    self,
                    dma:     &mut dma::Handle,
                    channel: Channel,
                    buffer:  Pin<Buffer>,
                ) -> Transfer<Self, Rx<$SPIX>, Channel, Buffer, dma::Ready>
                    where
                        Rx<$SPIX>:      dma::Target<Channel>,
                        Channel:        dma::Channel,
                        Buffer:         DerefMut + 'static,
                        Buffer::Target: AsMutSlice<Element=u8>,
                {
                    let num_words = buffer.len();
                    self.read_some(dma, channel, buffer, num_words)
                }

                pub fn read_some<Channel, Buffer>(
                    self,
                    dma:     &mut dma::Handle,
                    channel: Channel,
                    buffer:  Pin<Buffer>,
                    num_words: usize,
                ) -> Transfer<Self, Rx<$SPIX>, Channel, Buffer, dma::Ready>
                    where
                        Rx<$SPIX>:      dma::Target<Channel>,
                        Channel:        dma::Channel,
                        Buffer:         DerefMut + 'static,
                        Buffer::Target: AsMutSlice<Element=u8>,
                {
                    let token = Rx(PhantomData);
                    let address = &unsafe { &*$SPIX::ptr() }.dr as *const _ as u32;
                    // Safe, because the trait bounds of this method guarantee that the
                    // buffer can be written to.
                    let inner = unsafe {
                        dma::Transfer::new(
                            dma,
                            token,
                            channel,
                            buffer,
                            num_words,
                            address,
                            dma::Priority::high(),
                            dma::Direction::peripheral_to_memory(),
                            false,
                        )
                    };
                    Transfer {
                        target: self,
                        inner,
                    }
                }

                pub fn write_all<Channel, Buffer>(
                    self,
                    dma:     &mut dma::Handle,
                    channel: Channel,
                    buffer:  Pin<Buffer>,
                ) -> Transfer<Self, Tx<$SPIX>, Channel, Buffer, dma::Ready>
                    where
                        Tx<$SPIX>:      dma::Target<Channel>,
                        Channel:        dma::Channel,
                        Buffer:         Deref + 'static,
                        Buffer::Target: AsSlice<Element=u8>,
                {
                    let num_words = buffer.len();
                    self.write_some(dma, channel, buffer, num_words)
                }

                pub fn write_some<Channel, Buffer>(
                    self,
                    dma:     &mut dma::Handle,
                    channel: Channel,
                    buffer:  Pin<Buffer>,
                    num_words: usize,
                ) -> Transfer<Self, Tx<$SPIX>, Channel, Buffer, dma::Ready>
                    where
                        Tx<$SPIX>:      dma::Target<Channel>,
                        Channel:        dma::Channel,
                        Buffer:         Deref + 'static,
                        Buffer::Target: AsSlice<Element=u8>,
                {
                    let token = Tx(PhantomData);
                    let address = &unsafe { &*$SPIX::ptr() }.dr as *const _ as u32;
                    // Safe, because the trait bounds of this method guarantee that the
                    // buffer can be written to.
                    let inner = unsafe {
                        dma::Transfer::new(
                            dma,
                            token,
                            channel,
                            buffer,
                            num_words,
                            address,
                            dma::Priority::high(),
                            dma::Direction::memory_to_peripheral(),
                            false,
                        )
                    };
                    Transfer {
                        target: self,
                        inner,
                    }
                }
            }

            impl SpiExt<$SPIX> for $SPIX {
                fn spi<PINS, T>(self, pins: PINS, mode: Mode, freq: T, rcc: &mut Rcc) -> Spi<$SPIX, PINS>
                where
                    PINS: Pins<$SPIX>,
                    T: Into<Hertz>
                    {
                        Spi::$spiX(self, pins, mode, freq, rcc)
                    }
            }

            impl<PINS> hal::spi::FullDuplex<u8> for Spi<$SPIX, PINS> {
                type Error = Error;

                fn read(&mut self) -> nb::Result<u8, Error> {
                    let sr = self.spi.sr.read();

                    Err(if sr.ovr().bit_is_set() {
                        nb::Error::Other(Error::Overrun)
                    } else if sr.modf().bit_is_set() {
                        nb::Error::Other(Error::ModeFault)
                    } else if sr.crcerr().bit_is_set() {
                        nb::Error::Other(Error::Crc)
                    } else if sr.rxne().bit_is_set() {
                        // NOTE(read_volatile) read only 1 byte (the svd2rust API only allows
                        // reading a half-word)
                        return Ok(unsafe {
                            ptr::read_volatile(&self.spi.dr as *const _ as *const u8)
                        });
                    } else {
                        nb::Error::WouldBlock
                    })
                }

                fn send(&mut self, byte: u8) -> nb::Result<(), Error> {
                    let sr = self.spi.sr.read();

                    Err(if sr.ovr().bit_is_set() {
                        nb::Error::Other(Error::Overrun)
                    } else if sr.modf().bit_is_set() {
                        nb::Error::Other(Error::ModeFault)
                    } else if sr.crcerr().bit_is_set() {
                        nb::Error::Other(Error::Crc)
                    } else if sr.txe().bit_is_set() {
                        // NOTE(write_volatile) see note above
                        unsafe { ptr::write_volatile(&self.spi.dr as *const _ as *mut u8, byte) }
                        return Ok(());
                    } else {
                        nb::Error::WouldBlock
                    })
                }

            }

            impl<PINS> crate::hal::blocking::spi::transfer::Default<u8> for Spi<$SPIX, PINS> {}

            impl<PINS> crate::hal::blocking::spi::write::Default<u8> for Spi<$SPIX, PINS> {}
        )+
    }
}

spi! {
    SPI1: (spi1, apb2_clk),
}

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
spi! {
    SPI2: (spi2, apb1_clk),
}

/// Token used for DMA transfers
///
/// This is an implementation detail. The user doesn't have to deal with this
/// directly.
pub struct Tx<I>(PhantomData<I>);

/// Token used for DMA transfers
///
/// This is an implementation detail. The user doesn't have to deal with this
/// directly.
pub struct Rx<I>(PhantomData<I>);

/// Wrapper around a [`dma::Transfer`].
pub struct Transfer<Target, Token, Channel, Buffer, State> {
    target: Target,
    inner: dma::Transfer<Token, Channel, Buffer, State>,
}

impl<Target, Token, Channel, Buffer> Transfer<Target, Token, Channel, Buffer, dma::Ready>
where
    Token: dma::Target<Channel>,
    Channel: dma::Channel,
{
    /// Enables the provided interrupts
    ///
    /// This setting only affects this transfer. It doesn't affect transfer on
    /// other channels, or subsequent transfers on the same channel.
    pub fn enable_interrupts(&mut self, interrupts: dma::Interrupts) {
        self.inner.enable_interrupts(interrupts);
    }

    /// Start the DMA transfer
    ///
    /// Consumes this instance of `Transfer` and returns a new one, with its
    /// state changed to indicate that the transfer has been started.
    pub fn start(self) -> Transfer<Target, Token, Channel, Buffer, dma::Started> {
        Transfer {
            target: self.target,
            inner: self.inner.start(),
        }
    }
}

impl<Target, Token, Channel, Buffer> Transfer<Target, Token, Channel, Buffer, dma::Started>
where
    Channel: dma::Channel,
{
    /// Indicates whether the transfer is still ongoing
    pub fn is_active(&self) -> bool {
        self.inner.is_active()
    }

    /// Waits for the transfer to finish and returns the owned resources
    ///
    /// This function will busily wait until the transfer is finished. If you
    /// don't want this, please call this function only once you know that the
    /// transfer has finished.
    ///
    /// This function will return immediately, if [`Transfer::is_active`]
    /// returns `false`.
    pub fn wait(self) -> dma::TransferResourcesResult<Target, Channel, Buffer> {
        // Need to move `target` out of `self`, otherwise the closure captures
        // `self` completely.
        let target = self.target;

        let map_resources = |res: dma::TransferResources<_, _, _>| dma::TransferResources {
            target,
            channel: res.channel,
            buffer: res.buffer,
        };

        match self.inner.wait() {
            Ok(res) => Ok(map_resources(res)),
            Err((res, err)) => Err((map_resources(res), err)),
        }
    }
}
