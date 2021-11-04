//! I2C

use core::ops::Deref;

#[cfg(feature = "stm32l0x2")]
use core::{marker::PhantomData, ops::DerefMut, pin::Pin};

#[cfg(feature = "stm32l0x2")]
use as_slice::{AsMutSlice, AsSlice};

#[cfg(feature = "stm32l0x2")]
use crate::dma::{self, Buffer};
use crate::pac::i2c1::{
    cr2::{AUTOEND_A, RD_WRN_A},
    RegisterBlock,
};
use crate::rcc::{Enable, Rcc, Reset};
use cast::u8;
use embedded_time::rate::Hertz;

// I²C traits
use crate::hal::blocking::i2c::{Read, Write, WriteRead};

// I/O Imports
use crate::gpio::{AltMode, OpenDrain, Output};
#[cfg(feature = "io-STM32L051")]
use crate::{
    gpio::gpiob::{PB10, PB11, PB13, PB14, PB6, PB7, PB8, PB9},
    pac::{I2C1, I2C2},
};
#[cfg(feature = "io-STM32L021")]
use crate::{
    gpio::{
        gpioa::{PA10, PA13, PA4, PA9},
        gpiob::{PB6, PB7, PB8},
    },
    pac::I2C1,
};
#[cfg(feature = "io-STM32L071")]
use crate::{
    gpio::{
        gpioa::{PA10, PA8, PA9},
        gpiob::{PB10, PB11, PB13, PB14, PB4, PB6, PB7, PB8, PB9},
        gpioc::{PC0, PC1, PC9},
    },
    pac::{I2C1, I2C2, I2C3},
};
#[cfg(feature = "io-STM32L031")]
use crate::{
    gpio::{
        gpioa::{PA10, PA9},
        gpiob::{PB6, PB7, PB8, PB9},
    },
    pac::I2C1,
};

/// I2C abstraction
pub struct I2c<I2C, SDA, SCL> {
    i2c: I2C,
    sda: SDA,
    scl: SCL,
}

impl<I, SDA, SCL> I2c<I, SDA, SCL>
where
    I: Instance,
{
    pub fn new(i2c: I, sda: SDA, scl: SCL, freq: Hertz, rcc: &mut Rcc) -> Self
    where
        I: Instance,
        SDA: SDAPin<I>,
        SCL: SCLPin<I>,
    {
        sda.setup();
        scl.setup();

        // Enable clock for I2C
        I::enable(rcc);
        // Reset I2C
        I::reset(rcc);

        let freq = freq.0;

        assert!(freq <= 1_000_000);

        // TODO review compliance with the timing requirements of I2C
        // t_I2CCLK = 1 / PCLK1
        // t_PRESC  = (PRESC + 1) * t_I2CCLK
        // t_SCLL   = (SCLL + 1) * t_PRESC
        // t_SCLH   = (SCLH + 1) * t_PRESC
        //
        // t_SYNC1 + t_SYNC2 > 4 * t_I2CCLK
        // t_SCL ~= t_SYNC1 + t_SYNC2 + t_SCLL + t_SCLH
        let i2cclk = rcc.clocks.apb1_clk().0;
        let ratio = i2cclk / freq - 4;
        let (presc, scll, sclh, sdadel, scldel) = if freq >= 100_000 {
            // fast-mode or fast-mode plus
            // here we pick SCLL + 1 = 2 * (SCLH + 1)
            let presc = ratio / 387;

            let sclh = ((ratio / (presc + 1)) - 3) / 3;
            let scll = 2 * (sclh + 1) - 1;

            let (sdadel, scldel) = if freq > 400_000 {
                // fast-mode plus
                let sdadel = 0;
                let scldel = i2cclk / 4_000_000 / (presc + 1) - 1;

                (sdadel, scldel)
            } else {
                // fast-mode
                let sdadel = i2cclk / 8_000_000 / (presc + 1);
                let scldel = i2cclk / 2_000_000 / (presc + 1) - 1;

                (sdadel, scldel)
            };

            (presc, scll, sclh, sdadel, scldel)
        } else {
            // standard-mode
            // here we pick SCLL = SCLH
            let presc = ratio / 514;

            let sclh = ((ratio / (presc + 1)) - 2) / 2;
            let scll = sclh;

            let sdadel = i2cclk / 2_000_000 / (presc + 1);
            let scldel = i2cclk / 800_000 / (presc + 1) - 1;

            (presc, scll, sclh, sdadel, scldel)
        };

        let presc = u8(presc).unwrap();
        assert!(presc < 16);
        let scldel = u8(scldel).unwrap();
        assert!(scldel < 16);
        let sdadel = u8(sdadel).unwrap();
        assert!(sdadel < 16);
        let sclh = u8(sclh).unwrap();
        let scll = u8(scll).unwrap();

        // Configure for "fast mode" (400 KHz)
        i2c.timingr.write(|w| {
            w.presc().bits(presc);
            w.scll().bits(scll);
            w.sclh().bits(sclh);
            w.sdadel().bits(sdadel);
            w.scldel().bits(scldel)
        });

        i2c.cr1.write(|w| {
            // Enable DMA reception
            w.rxdmaen().set_bit();
            // Enable DMA transmission
            w.txdmaen().set_bit();
            // Enable peripheral
            w.pe().set_bit()
        });

        I2c { i2c, sda, scl }
    }

    pub fn release(self) -> (I, SDA, SCL) {
        (self.i2c, self.sda, self.scl)
    }

    fn check_errors(&self) -> Result<(), Error> {
        let isr = self.i2c.isr.read();
        if isr.berr().bit_is_set() {
            self.i2c.icr.write(|w| w.berrcf().set_bit());
            Err(Error::BusError)
        } else if isr.arlo().bit_is_set() {
            self.i2c.icr.write(|w| w.arlocf().set_bit());
            Err(Error::ArbitrationLost)
        } else if isr.nackf().bit_is_set() {
            self.i2c.icr.write(|w| w.nackcf().set_bit());
            Err(Error::Nack)
        } else {
            Ok(())
        }
    }

    fn start_transfer(&mut self, addr: u8, len: usize, direction: RD_WRN_A, autoend: AUTOEND_A) {
        // Ensure that TX/RX buffers are empty
        self.i2c.isr.write(|w| w.txe().set_bit());
        while self.i2c.isr.read().rxne().bit_is_set() {
            self.i2c.rxdr.read();
        }

        self.i2c.cr2.write(|w| {
            // Start transfer
            w.start().set_bit();
            // Set number of bytes to transfer
            w.nbytes().bits(len as u8);
            // Set address to transfer to/from
            w.sadd().bits((addr << 1) as u16);
            // Set transfer direction
            w.rd_wrn().variant(direction);
            // should we end the transfer automatically?
            w.autoend().variant(autoend)
        });
    }

    fn send_byte(&self, byte: u8) -> Result<(), Error> {
        // Wait until we're ready for sending
        while self.i2c.isr.read().txe().bit_is_clear() {
            self.check_errors()?;
        }

        // Push out a byte of data
        self.i2c.txdr.write(|w| w.txdata().bits(byte));

        // check for any errors
        self.check_errors()
    }

    fn recv_byte(&self) -> Result<u8, Error> {
        while self.i2c.isr.read().rxne().bit_is_clear() {
            self.check_errors()?;
        }

        let value = self.i2c.rxdr.read().rxdata().bits();
        Ok(value)
    }

    #[cfg(feature = "stm32l0x2")]
    pub fn write_all<Channel, Buffer>(
        self,
        dma: &mut dma::Handle,
        channel: Channel,
        address: u8,
        buffer: Pin<Buffer>,
    ) -> Transfer<Self, Tx<I>, Channel, Buffer, dma::Ready>
    where
        Tx<I>: dma::Target<Channel>,
        Channel: dma::Channel,
        Buffer: Deref + 'static,
        Buffer::Target: AsSlice<Element = u8>,
    {
        let num_words = buffer.len();
        self.write_some(dma, channel, address, buffer, num_words)
    }

    #[cfg(feature = "stm32l0x2")]
    pub fn write_some<Channel, Buffer>(
        mut self,
        dma: &mut dma::Handle,
        channel: Channel,
        address: u8,
        buffer: Pin<Buffer>,
        num_words: usize,
    ) -> Transfer<Self, Tx<I>, Channel, Buffer, dma::Ready>
    where
        Tx<I>: dma::Target<Channel>,
        Channel: dma::Channel,
        Buffer: Deref + 'static,
        Buffer::Target: AsSlice<Element = u8>,
    {
        assert!(buffer.len() >= num_words);
        self.start_transfer(
            address,
            buffer.as_slice().len(),
            RD_WRN_A::WRITE,
            AUTOEND_A::AUTOMATIC,
        );

        // This token represents the transmission capability of I2C and this is
        // what the `dma::Target` trait is implemented for. It can't be
        // implemented for `I2c` itself, as that would allow for the user to
        // pass, for example, a channel that can do I2C RX to `write_all`.
        //
        // Theoretically, one could create both `Rx` and `Tx` at the same time,
        // or create multiple tokens of the same type, and use that to create
        // multiple simultaneous DMA transfers, which would be wrong and is not
        // supported by the I2C peripheral. We prevent that by only ever
        // creating an `Rx` or `Tx` token while we have ownership of `I2c`, and
        // dropping the token before returning ownership of `I2c` ot the user.
        let token = Tx(PhantomData);

        // Safe, because we're only taking the address of a register.
        let address = &unsafe { &*I::ptr() }.txdr as *const _ as u32;

        // Safe, because the trait bounds of this method guarantee that the
        // buffer can be read from.
        let transfer = unsafe {
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
            inner: transfer,
        }
    }

    #[cfg(feature = "stm32l0x2")]
    pub fn read_all<Channel, Buffer>(
        self,
        dma: &mut dma::Handle,
        channel: Channel,
        address: u8,
        buffer: Pin<Buffer>,
    ) -> Transfer<Self, Rx<I>, Channel, Buffer, dma::Ready>
    where
        Rx<I>: dma::Target<Channel>,
        Channel: dma::Channel,
        Buffer: DerefMut + 'static,
        Buffer::Target: AsMutSlice<Element = u8>,
    {
        let num_words = buffer.len();
        self.read_some(dma, channel, address, buffer, num_words)
    }

    #[cfg(feature = "stm32l0x2")]
    pub fn read_some<Channel, Buffer>(
        mut self,
        dma: &mut dma::Handle,
        channel: Channel,
        address: u8,
        buffer: Pin<Buffer>,
        num_words: usize,
    ) -> Transfer<Self, Rx<I>, Channel, Buffer, dma::Ready>
    where
        Rx<I>: dma::Target<Channel>,
        Channel: dma::Channel,
        Buffer: DerefMut + 'static,
        Buffer::Target: AsMutSlice<Element = u8>,
    {
        assert!(buffer.len() >= num_words);
        self.start_transfer(
            address,
            buffer.as_slice().len(),
            RD_WRN_A::READ,
            AUTOEND_A::AUTOMATIC,
        );

        // See explanation of tokens in `write_all`.
        let token = Rx(PhantomData);

        // Safe, because we're only taking the address of a register.
        let address = &unsafe { &*I::ptr() }.rxdr as *const _ as u32;

        let num_words = buffer.len();
        // Safe, because the trait bounds of this method guarantee that the
        // buffer can be written to.
        let transfer = unsafe {
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
            inner: transfer,
        }
    }
}

impl<I, SDA, SCL> WriteRead for I2c<I, SDA, SCL>
where
    I: Instance,
{
    type Error = Error;

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        let writing = !bytes.is_empty();
        let reading = !buffer.is_empty();

        // wait for i2c device to be available
        while self.i2c.isr.read().busy().is_busy() {
            self.check_errors()?;
        }

        // if we are writing bytes
        if writing {
            // if the previous write has failed, we need to flush the TX
            // buffer to prevent sending old data
            self.i2c.isr.write(|w| w.txe().set_bit());

            if reading {
                self.start_transfer(addr, bytes.len(), RD_WRN_A::WRITE, AUTOEND_A::SOFTWARE);
            } else {
                self.start_transfer(addr, bytes.len(), RD_WRN_A::WRITE, AUTOEND_A::AUTOMATIC);
            }

            // Send bytes
            for c in bytes {
                self.send_byte(*c)?;
            }

            // if we are going to read afterwards, we need to wait for
            // the tx to complete
            if reading {
                while self.i2c.isr.read().tc().is_not_complete() {
                    self.check_errors()?;
                }
            }
        }

        if reading {
            // force a read of the rx data register, so that we dont
            // get stale data from the last transaction (if there is
            // anything left over)
            self.i2c.rxdr.read();

            //send a new start condition and transfer
            self.start_transfer(addr, buffer.len(), RD_WRN_A::READ, AUTOEND_A::AUTOMATIC);

            // Receive bytes into buffer
            for c in buffer {
                *c = self.recv_byte()?;
            }
        }

        Ok(())
    }
}

impl<I, SDA, SCL> Write for I2c<I, SDA, SCL>
where
    I: Instance,
{
    type Error = Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.write_read(addr, bytes, &mut [])
    }
}

impl<I, SDA, SCL> Read for I2c<I, SDA, SCL>
where
    I: Instance,
{
    type Error = Error;

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.write_read(addr, &[], buffer)
    }
}

pub trait Instance: Deref<Target = RegisterBlock> + Enable + Reset {
    fn ptr() -> *const RegisterBlock;
}

// I2C SDA pin
pub trait SDAPin<I2C> {
    fn setup(&self);
}

// I2C SCL pin
pub trait SCLPin<I2C> {
    fn setup(&self);
}

// I2C error
#[derive(Debug)]
pub enum Error {
    Overrun,
    Nack,
    PECError,
    BusError,
    ArbitrationLost,
}

pub trait I2cExt<I2C> {
    fn i2c<SDA, SCL>(self, sda: SDA, scl: SCL, freq: Hertz, rcc: &mut Rcc) -> I2c<I2C, SDA, SCL>
    where
        SDA: SDAPin<I2C>,
        SCL: SCLPin<I2C>;
}

macro_rules! i2c {
    ($I2CX:ident,
        sda: [ $(($PSDA:ty, $afsda:expr),)+ ],
        scl: [ $(($PSCL:ty, $afscl:expr),)+ ],
    ) => {
        $(
            impl SDAPin<$I2CX> for $PSDA {
                fn setup(&self) {
                    self.set_alt_mode($afsda)
                }
            }
        )+

        $(
            impl SCLPin<$I2CX> for $PSCL {
                fn setup(&self) {
                    self.set_alt_mode($afscl)
                }
            }
        )+

        impl I2cExt<$I2CX> for $I2CX {
            fn i2c<SDA, SCL>(
                self,
                sda: SDA,
                scl: SCL,
                freq: Hertz,
                rcc: &mut Rcc,
            ) -> I2c<$I2CX, SDA, SCL>
            where
                SDA: SDAPin<$I2CX>,
                SCL: SCLPin<$I2CX>,
            {
                I2c::new(self, sda, scl, freq, rcc)
            }
        }

        impl Instance for $I2CX {
            fn ptr() -> *const RegisterBlock {
                $I2CX::ptr()
            }
        }
    };
}

#[cfg(feature = "io-STM32L021")]
i2c!(
    I2C1,
    sda: [
        (PA10<Output<OpenDrain>>, AltMode::AF1),
        (PA13<Output<OpenDrain>>, AltMode::AF3),
        (PB7<Output<OpenDrain>>, AltMode::AF1),
    ],
    scl: [
        (PA4<Output<OpenDrain>>, AltMode::AF3),
        (PA9<Output<OpenDrain>>, AltMode::AF1),
        (PB6<Output<OpenDrain>>, AltMode::AF1),
        (PB8<Output<OpenDrain>>, AltMode::AF4),
    ],
);

#[cfg(feature = "io-STM32L031")]
i2c!(
    I2C1,
    sda: [
        (PA10<Output<OpenDrain>>, AltMode::AF1),
        (PB7<Output<OpenDrain>>, AltMode::AF1),
        (PB9<Output<OpenDrain>>, AltMode::AF4),
    ],
    scl: [
        (PA9<Output<OpenDrain>>, AltMode::AF1),
        (PB6<Output<OpenDrain>>, AltMode::AF1),
        (PB8<Output<OpenDrain>>, AltMode::AF4),
    ],
);

#[cfg(feature = "io-STM32L051")]
i2c!(
    I2C1,
    sda: [
        (PB7<Output<OpenDrain>>, AltMode::AF1),
        (PB9<Output<OpenDrain>>, AltMode::AF4),
    ],
    scl: [
        (PB6<Output<OpenDrain>>, AltMode::AF1),
        (PB8<Output<OpenDrain>>, AltMode::AF4),
    ],
);

#[cfg(feature = "io-STM32L051")]
i2c!(
    I2C2,
    sda: [
        (PB11<Output<OpenDrain>>, AltMode::AF6),
        (PB14<Output<OpenDrain>>, AltMode::AF5),
    ],
    scl: [
        (PB10<Output<OpenDrain>>, AltMode::AF6),
        (PB13<Output<OpenDrain>>, AltMode::AF5),
    ],
);

#[cfg(feature = "io-STM32L071")]
i2c!(
    I2C1,
    sda: [
        (PA10<Output<OpenDrain>>, AltMode::AF6),
        (PB7<Output<OpenDrain>>, AltMode::AF1),
        (PB9<Output<OpenDrain>>, AltMode::AF4),
    ],
    scl: [
        (PA9<Output<OpenDrain>>, AltMode::AF6),
        (PB6<Output<OpenDrain>>, AltMode::AF1),
        (PB8<Output<OpenDrain>>, AltMode::AF4),
    ],
);

#[cfg(feature = "io-STM32L071")]
i2c!(
    I2C2,
    sda: [
        (PB11<Output<OpenDrain>>, AltMode::AF6),
        (PB14<Output<OpenDrain>>, AltMode::AF5),
    ],
    scl: [
        (PB10<Output<OpenDrain>>, AltMode::AF6),
        (PB13<Output<OpenDrain>>, AltMode::AF5),
    ],
);

#[cfg(feature = "io-STM32L071")]
i2c!(
    I2C3,
    sda: [
        (PB4<Output<OpenDrain>>, AltMode::AF7),
        (PC1<Output<OpenDrain>>, AltMode::AF7),
        (PC9<Output<OpenDrain>>, AltMode::AF7),
    ],
    scl: [
        (PA8<Output<OpenDrain>>, AltMode::AF7),
        (PC0<Output<OpenDrain>>, AltMode::AF7),
    ],
);

/// Token used for DMA transfers
///
/// This is an implementation detail. The user doesn't have to deal with this
/// directly.
#[cfg(feature = "stm32l0x2")]
pub struct Tx<I>(PhantomData<I>);

/// Token used for DMA transfers
///
/// This is an implementation detail. The user doesn't have to deal with this
/// directly.
#[cfg(feature = "stm32l0x2")]
pub struct Rx<I>(PhantomData<I>);

/// I2C-specific wrapper around [`dma::Transfer`]
#[cfg(feature = "stm32l0x2")]
pub struct Transfer<Target, Token, Channel, Buffer, State> {
    target: Target,
    inner: dma::Transfer<Token, Channel, Buffer, State>,
}

#[cfg(feature = "stm32l0x2")]
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

#[cfg(feature = "stm32l0x2")]
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
    pub fn wait(
        self,
    ) -> Result<
        dma::TransferResources<Target, Channel, Buffer>,
        (dma::TransferResources<Target, Channel, Buffer>, dma::Error),
    > {
        // Need to move `target` out of `self`, otherwise the closure captures
        // `self` completely.
        let target = self.target;

        let map_resources = |res: dma::TransferResources<_, _, _>| dma::TransferResources {
            target: target,
            channel: res.channel,
            buffer: res.buffer,
        };

        match self.inner.wait() {
            Ok(res) => Ok(map_resources(res)),
            Err((res, err)) => Err((map_resources(res), err)),
        }
    }
}
