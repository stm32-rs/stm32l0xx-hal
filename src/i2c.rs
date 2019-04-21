//! I2C
use hal::blocking::i2c::{Read, Write, WriteRead};

use crate::gpio::gpiob::{PB6, PB7};
use crate::gpio::{AltMode, OpenDrain, Output};
use crate::prelude::*;
use crate::rcc::Rcc;
use crate::stm32::{I2C1};
use crate::time::Hertz;

/// I2C abstraction
pub struct I2c<I2C, PINS> {
    i2c: I2C,
    pins: PINS,
}

pub trait Pins<I2c> {
    fn setup(&self);
}

impl Pins<I2C1> for (PB6<Output<OpenDrain>>, PB7<Output<OpenDrain>>) {
    fn setup(&self) {
        self.0.set_alt_mode(AltMode::I2C);
        self.1.set_alt_mode(AltMode::I2C);
    }
}

#[derive(Debug)]
pub enum Error {
    OVERRUN,
    NACK,
}

macro_rules! i2c {
    ($I2CX:ident, $i2cx:ident, $i2cxen:ident, $i2crst:ident, $I2cxExt:ident) => {
        impl<PINS> I2c<$I2CX, PINS> {
            pub fn $i2cx(i2c: $I2CX, pins: PINS, speed: Hertz, rcc: &mut Rcc) -> Self
            where
                PINS: Pins<$I2CX>,
            {
                pins.setup();
                // let speed: Hertz = speed.into();

                // // Enable clock for I2C
                // rcc.rb.apb1enr.modify(|_, w| w.$i2cxen().set_bit());

                // // Reset I2C
                // rcc.rb.apb1rstr.modify(|_, w| w.$i2crst().set_bit());
                // rcc.rb.apb1rstr.modify(|_, w| w.$i2crst().clear_bit());

                // // Make sure the I2C unit is disabled so we can configure it
                // i2c.cr1.modify(|_, w| w.pe().clear_bit());

                // // Calculate settings for I2C speed modes
                // let clock = rcc.clocks.apb1_clk().0;
                // let freq = clock / 1_000_000;
                // assert!(freq >= 2 && freq <= 50);

                // TODO: Replace with TIMINGR configuration
                /*
                // Configure bus frequency into I2C peripheral
                //i2c.cr2.write(|w| unsafe { w.freq().bits(freq as u8) });

                //let trise = if speed <= 100_u32.khz().into() {
                //    freq + 1
                //} else {
                //    (freq * 300) / 1000 + 1
                //};

                // Configure correct rise times
                //i2c.trise.write(|w| w.trise().bits(trise as u8));

                // I2C clock control calculation
                if speed <= 100_u32.khz().into() {
                    let ccr = {
                        let ccr = clock / (speed.0 * 2);
                        if ccr < 4 {
                            4
                        } else {
                            ccr
                        }
                    };

                    // Set clock to standard mode with appropriate parameters for selected speed
                    i2c.ccr.write(|w| unsafe {
                        w.f_s()
                            .clear_bit()
                            .duty()
                            .clear_bit()
                            .ccr()
                            .bits(ccr as u16)
                    });
                } else {
                    const DUTYCYCLE: u8 = 0;
                    if DUTYCYCLE == 0 {
                        let ccr = clock / (speed.0 * 3);
                        let ccr = if ccr < 1 { 1 } else { ccr };

                        // Set clock to fast mode with appropriate parameters for selected speed (2:1 duty cycle)
                        i2c.ccr.write(|w| unsafe {
                            w.f_s().set_bit().duty().clear_bit().ccr().bits(ccr as u16)
                        });
                    } else {
                        let ccr = clock / (speed.0 * 25);
                        let ccr = if ccr < 1 { 1 } else { ccr };

                        // Set clock to fast mode with appropriate parameters for selected speed (16:9 duty cycle)
                        i2c.ccr.write(|w| unsafe {
                            w.f_s().set_bit().duty().set_bit().ccr().bits(ccr as u16)
                        });
                    }
                }
                */
                // Enable the I2C processing
                i2c.cr1.modify(|_, w| w.pe().set_bit());

                I2c { i2c, pins }
            }

            pub fn release(self) -> ($I2CX, PINS) {
                (self.i2c, self.pins)
            }

            fn send_byte(&self, byte: u8) -> Result<(), Error> {
                // Wait until we're ready for sending
                while self.i2c.isr.read().txe().bit_is_clear() {}

                // Push out a byte of data
                self.i2c.txdr.write(|w| unsafe { w.txdata().bits(byte) });

                // While until byte is transferred
                while {
                    let sr1 = self.i2c.isr.read();

                    // If we received a NACK, then this is an error
                    if sr1.nackf().bit_is_set() {
                        return Err(Error::NACK);
                    }

                    sr1.tcr().bit_is_clear()
                } {}

                Ok(())
            }

            fn recv_byte(&self) -> Result<u8, Error> {
                while self.i2c.isr.read().rxne().bit_is_clear() {}
                //let value = self.i2c.dr.read().bits() as u8;
                //Ok(value)
                Ok(0)
            }
        }

        impl<PINS> WriteRead for I2c<$I2CX, PINS> {
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

        impl<PINS> Write for I2c<$I2CX, PINS> {
            type Error = Error;

            fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
                // Send a START condition
                self.i2c.cr2.modify(|_, w| w.start().set_bit());

                // // Wait until START condition was generated
                // while {
                //     let sr1 = self.i2c.isr.read();
                //     sr1.sb().bit_is_clear()
                // } {}

                // // Also wait until signalled we're master and everything is waiting for us
                // while {
                //     let sr2 = self.i2c.sr2.read();
                //     sr2.msl().bit_is_clear() && sr2.busy().bit_is_clear()
                // } {}

                // // Set up current address, we're trying to talk to
                // self.i2c
                //     .dr
                //     .write(|w| unsafe { w.bits(u32::from(addr) << 1) });

                // // Wait until address was sent
                // while {
                //     let sr1 = self.i2c.sr1.read();
                //     sr1.addr().bit_is_clear()
                // } {}

                // // Clear condition by reading SR2
                // self.i2c.sr2.read();

                // // Send bytes
                // for c in bytes {
                //     self.send_byte(*c)?;
                // }

                // Fallthrough is success
                Ok(())
            }
        }

        impl<PINS> Read for I2c<$I2CX, PINS> {
            type Error = Error;

            fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
                // Send a START condition and set ACK bit
                // self.i2c
                //     .cr1
                //     .modify(|_, w| w.start().set_bit().ack().set_bit());

                // // Wait until START condition was generated
                // while {
                //     let sr1 = self.i2c.sr1.read();
                //     sr1.sb().bit_is_clear()
                // } {}

                // // Also wait until signalled we're master and everything is waiting for us
                // while {
                //     let sr2 = self.i2c.sr2.read();
                //     sr2.msl().bit_is_clear() && sr2.busy().bit_is_clear()
                // } {}

                // // Set up current address, we're trying to talk to
                // self.i2c
                //     .dr
                //     .write(|w| unsafe { w.bits((u32::from(addr) << 1) + 1) });

                // // Wait until address was sent
                // while {
                //     let sr1 = self.i2c.sr1.read();
                //     sr1.addr().bit_is_clear()
                // } {}

                // // Clear condition by reading SR2
                // self.i2c.sr2.read();

                // // Receive bytes into buffer
                // for c in buffer {
                //     *c = self.recv_byte()?;
                // }

                // // Send STOP condition
                // self.i2c.cr1.modify(|_, w| w.stop().set_bit());

                // Fallthrough is success
                Ok(())
            }
        }

        pub trait $I2cxExt {
            fn i2c<PINS, T>(self, pins: PINS, speed: T, rcc: &mut Rcc) -> I2c<$I2CX, PINS>
            where
                PINS: Pins<$I2CX>,
                T: Into<Hertz>;
        }

        impl $I2cxExt for $I2CX {
            fn i2c<PINS, T>(self, pins: PINS, speed: T, rcc: &mut Rcc) -> I2c<$I2CX, PINS>
            where
                PINS: Pins<$I2CX>,
                T: Into<Hertz>,
            {
                I2c::$i2cx(self, pins, speed.into(), rcc)
            }
        }
    };
}

i2c!(I2C1, i2c1, i2c1en, i2c1rst, I2c1Ext);
