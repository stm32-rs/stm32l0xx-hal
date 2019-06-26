//! I2C
use crate::hal::blocking::i2c::{Read, Write, WriteRead};

use cast::u8;
use crate::gpio::gpioa::{PA4, PA9, PA10, PA13};
use crate::gpio::gpiob::{PB6, PB7};
use crate::gpio::{AltMode, OpenDrain, Output};
use crate::pac::I2C1;
use crate::rcc::Rcc;
use crate::time::Hertz;


/// I2C abstraction
pub struct I2c<I2C, SDA, SCL> {
    i2c: I2C,
    sda: SDA,
    scl: SCL,
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
    ($I2CX:ident, $i2cx:ident, $i2cxen:ident, $i2crst:ident,
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
                I2c::$i2cx(self, sda, scl, freq, rcc)
            }
        }

        impl<SDA, SCL> I2c<$I2CX, SDA, SCL> {
            pub fn $i2cx(i2c: $I2CX, sda: SDA, scl: SCL, freq: Hertz, rcc: &mut Rcc) -> Self
            where
                SDA: SDAPin<$I2CX>,
                SCL: SCLPin<$I2CX>,
            {

                sda.setup();
                scl.setup();

                // Enable clock for I2C
                rcc.rb.apb1enr.modify(|_, w| w.$i2cxen().set_bit());

                // Reset I2C
                rcc.rb.apb1rstr.modify(|_, w| w.$i2crst().set_bit());
                rcc.rb.apb1rstr.modify(|_, w| w.$i2crst().clear_bit());

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

            pub fn release(self) -> ($I2CX, SDA, SCL) {
                (self.i2c, self.sda, self.scl)
            }

            fn send_byte(&self, byte: u8) -> Result<(), Error> {
                // Wait until we're ready for sending
                while self.i2c.isr.read().txe().bit_is_clear() {}

                // Push out a byte of data
                self.i2c.txdr.write(|w| w.txdata().bits(byte));

                // While until byte is transferred
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
                    return Ok(())
                }
            }

            fn recv_byte(&self) -> Result<u8, Error> {
                while self.i2c.isr.read().rxne().bit_is_clear() {}

                let value = self.i2c.rxdr.read().rxdata().bits();
                Ok(value)
            }
        }

        impl<SDA, SCL> WriteRead for I2c<$I2CX, SDA, SCL> {
            type Error = Error;

            fn write_read(
                &mut self,
                addr: u8,
                bytes: &[u8],
                buffer: &mut [u8],
            ) -> Result<(), Self::Error> {
                self.write(addr, bytes)?;
                self.read(addr, buffer)?;

                Ok(())
            }
        }

        impl<SDA, SCL> Write for I2c<$I2CX, SDA, SCL> {
            type Error = Error;

            fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
                self.i2c.cr2.modify(|_, w|
                    w.start()
                        .set_bit()
                        .nbytes()
                        .bits(bytes.len() as u8)
                        .sadd()
                        .bits((addr << 1) as u16)
                        .rd_wrn()
                        .clear_bit()
                        .autoend()
                        .set_bit()
                );

                while self.i2c.isr.read().busy().bit_is_clear() {}

                // Send bytes
                for c in bytes {
                    self.send_byte(*c)?;
                }

                Ok(())
            }
        }

        impl<SDA, SCL> Read for I2c<$I2CX, SDA, SCL> {
            type Error = Error;

            fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
                self.i2c.cr2.modify(|_, w|
                    w.start()
                        .set_bit()
                        .nbytes()
                        .bits(buffer.len() as u8)
                        .sadd()
                        .bits((addr << 1) as u16)
                        .autoend()
                        .set_bit()
                );

                // Wait until address was sent
                while self.i2c.isr.read().busy().bit_is_clear() {}

                // Receive bytes into buffer
                for c in buffer {
                    *c = self.recv_byte()?;
                }
                Ok(())
            }
        }
    };
}

i2c!(
    I2C1,
    i2c1,
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
