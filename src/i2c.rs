//! I2C

use core::{
    marker::PhantomData,
    ops::Deref,
};

use crate::hal::blocking::i2c::{Read, Write, WriteRead};

use crate::gpio::gpioa::{PA10, PA9};
use crate::gpio::gpiob::{PB6, PB7};
use crate::gpio::{AltMode, OpenDrain, Output};
use crate::pac::{
    i2c1::{
        RegisterBlock,
        cr2::RD_WRNW,
    },
    I2C1,
};
use crate::rcc::Rcc;
use crate::time::Hertz;
use cast::u8;

#[cfg(feature = "stm32l0x1")]
use crate::gpio::gpioa::{PA13, PA4};

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
use crate::{
    gpio::{
        gpioa::PA8,
        gpiob::{PB10, PB11, PB13, PB14, PB4, PB8, PB9},
        gpioc::{PC0, PC1},
    },
    pac::{I2C2, I2C3},
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

        i2c.initialize(rcc);

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
            w.presc()
                .bits(presc)
                .scll()
                .bits(scll)
                .sclh()
                .bits(sclh)
                .sdadel()
                .bits(sdadel)
                .scldel()
                .bits(scldel)
        });

        // Enable the peripheral
        i2c.cr1.write(|w| w.pe().set_bit());

        I2c { i2c, sda, scl }
    }

    pub fn release(self) -> (I, SDA, SCL) {
        (self.i2c, self.sda, self.scl)
    }

    fn start_transfer(&mut self, addr: u8, len: usize, direction: RD_WRNW) {
        self.i2c.cr2.write(|w|
            w
                // Start transfer
                .start().set_bit()
                // Set number of bytes to transfer
                .nbytes().bits(len as u8)
                // Set address to transfer to/from
                .sadd().bits((addr << 1) as u16)
                // Set transfer direction
                .rd_wrn().variant(direction)
                // End transfer once all bytes have been written
                .autoend().set_bit()
        );
    }

    fn send_byte(&self, byte: u8) -> Result<(), Error> {
        // Wait until we're ready for sending
        while self.i2c.isr.read().txe().bit_is_clear() {}

        // Push out a byte of data
        self.i2c.txdr.write(|w| w.txdata().bits(byte));

        // Wait until byte is transferred
        loop {
            let isr = self.i2c.isr.read();
            if isr.berr().bit_is_set() {
                self.i2c.icr.write(|w| w.berrcf().set_bit());
                return Err(Error::BusError);
            } else if isr.arlo().bit_is_set() {
                self.i2c.icr.write(|w| w.arlocf().set_bit());
                return Err(Error::ArbitrationLost);
            } else if isr.nackf().bit_is_set() {
                self.i2c.icr.write(|w| w.nackcf().set_bit());
                return Err(Error::Nack);
            }
            return Ok(());
        }
    }

    fn recv_byte(&self) -> Result<u8, Error> {
        while self.i2c.isr.read().rxne().bit_is_clear() {}

        let value = self.i2c.rxdr.read().rxdata().bits();
        Ok(value)
    }
}

impl<I, SDA, SCL> WriteRead for I2c<I, SDA, SCL>
where
    I: Instance,
{
    type Error = Error;

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.write(addr, bytes)?;
        self.read(addr, buffer)?;

        Ok(())
    }
}

impl<I, SDA, SCL> Write for I2c<I, SDA, SCL>
where
    I: Instance,
{
    type Error = Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        while self.i2c.isr.read().busy().is_busy() {}

        self.start_transfer(addr, bytes.len(), RD_WRNW::WRITE);

        // Send bytes
        for c in bytes {
            self.send_byte(*c)?;
        }

        Ok(())
    }
}

impl<I, SDA, SCL> Read for I2c<I, SDA, SCL>
where
    I: Instance,
{
    type Error = Error;

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        while self.i2c.isr.read().busy().is_busy() {}

        self.start_transfer(addr, buffer.len(), RD_WRNW::READ);

        // Receive bytes into buffer
        for c in buffer {
            *c = self.recv_byte()?;
        }
        Ok(())
    }
}

pub trait Instance: Deref<Target = RegisterBlock> {
    fn initialize(&self, rcc: &mut Rcc);
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
    ($I2CX:ident, $i2cxen:ident, $i2crst:ident,
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
            fn initialize(&self, rcc: &mut Rcc) {
                // Enable clock for I2C
                rcc.rb.apb1enr.modify(|_, w| w.$i2cxen().set_bit());

                // Reset I2C
                rcc.rb.apb1rstr.modify(|_, w| w.$i2crst().set_bit());
                rcc.rb.apb1rstr.modify(|_, w| w.$i2crst().clear_bit());
            }
        }
    };
}

#[cfg(feature = "stm32l0x1")]
i2c!(
    I2C1,
    i2c1en,
    i2c1rst,
    sda: [
        (PB7<Output<OpenDrain>>, AltMode::AF1),
        (PA10<Output<OpenDrain>>, AltMode::AF1),
        (PA13<Output<OpenDrain>>, AltMode::AF3),
    ],
    scl: [
        (PB6<Output<OpenDrain>>, AltMode::AF1),
        (PA9<Output<OpenDrain>>, AltMode::AF1),
        (PA4<Output<OpenDrain>>, AltMode::AF3),
    ],
);

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
i2c!(
    I2C1,
    i2c1en,
    i2c1rst,
    sda: [
        (PA10<Output<OpenDrain>>, AltMode::AF6),
        (PB7<Output<OpenDrain>>,  AltMode::AF1),
        (PB9<Output<OpenDrain>>,  AltMode::AF4),
    ],
    scl: [
        (PA9<Output<OpenDrain>>, AltMode::AF6),
        (PB6<Output<OpenDrain>>, AltMode::AF1),
        (PB8<Output<OpenDrain>>, AltMode::AF4),
    ],
);

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
i2c!(
    I2C2,
    i2c2en,
    i2c2rst,
    sda: [
        (PB11<Output<OpenDrain>>, AltMode::AF6),
        (PB14<Output<OpenDrain>>, AltMode::AF5),
    ],
    scl: [
        (PB10<Output<OpenDrain>>, AltMode::AF6),
        (PB13<Output<OpenDrain>>, AltMode::AF5),
    ],
);

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
i2c!(
    I2C3,
    i2c3en,
    i2c3rst,
    sda: [
        (PB4<Output<OpenDrain>>, AltMode::AF7),
        (PC1<Output<OpenDrain>>, AltMode::AF7),
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
pub struct Tx<I>(PhantomData<I>);

/// Token used for DMA transfers
///
/// This is an implementation detail. The user doesn't have to deal with this
/// directly.
pub struct Rx<I>(PhantomData<I>);
