use crate::gpio::gpioa::*;
#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
use crate::gpio::gpiob::*;

use crate::gpio::{AltMode, Analog};
use crate::hal;
use crate::pac::SPI1;
#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
use crate::pac::SPI2;
use crate::rcc::Rcc;
use crate::time::Hertz;
use core::ptr;
use nb;

pub use hal::spi::{Mode, Phase, Polarity, MODE_0, MODE_1, MODE_2, MODE_3};

/// SPI error
#[derive(Debug)]
pub enum Error {
    Busy,
    FrameError,
    /// Overrun occurred
    Overrun,
    /// Mode fault occurred
    ModeFault,
    /// CRC error
    Crc,
    #[doc(hidden)]
    _Extensible,
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
            [PA5<Analog>, AltMode::AF0]
        ]
        MISO: [
            [NoMiso, None],
            [PA6<Analog>, AltMode::AF0]
        ]
        MOSI: [
            [NoMosi, None],
            [PA7<Analog>, AltMode::AF0]
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
    ($($SPIX:ident: ($spiX:ident, $apbXenr:ident, $spiXen:ident, $pclkX:ident),)+) => {
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
                    rcc.rb.$apbXenr.modify(|_, w| w.$spiXen().set_bit());

                    // disable SS output
                    spi.cr2.write(|w| w.ssoe().clear_bit());

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
    SPI1: (spi1, apb2enr, spi1en, apb2_clk),
}

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
spi! {
    SPI2: (spi2, apb1enr, spi2en, apb1_clk),
}
