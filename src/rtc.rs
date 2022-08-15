//! Interface to the Real-time clock (RTC) peripheral.
//!
//! The real-time clock (RTC) is an independent BCD timer/counter. The RTC
//! provides a time- of-day clock/calendar with programmable alarm interrupts.
//!
//! ## Date Types
//!
//! The date/time types (`NaiveDate`, `NaiveTime` and `NaiveDateTime`), are
//! re-exported from [`rtcc`](https://docs.rs/rtcc/), which in turn re-exports
//! them from [`chrono`](https://docs.rs/chrono/).
//!
//! To use methods like `.year()` or `.hour()`, you may need to import the
//! `Datelike` or `Timelike` traits.
//!
//! ## Valid Date Range
//!
//! The RTC only supports two-digit years (00-99). The value 00 corresponds to
//! the year 2000 (not 1970!). Additionally, the year 00 cannot be used because
//! this value is the RTC domain reset default value. This means that dates in
//! the range 2001-01-01 to 2099-12-31 can be represented.
//!
//! ## More Information
//!
//! See STM32L0x2 reference manual, chapter 26 or STM32L0x1 reference manual,
//! chapter 22 for more details.

use core::convert::TryInto;

use embedded_time::rate::Extensions;
use void::Void;

use crate::{
    hal::timer::{self, Cancel as _},
    pac,
    pwr::PWR,
    rcc::Rcc,
};

#[doc(no_inline)]
pub use rtcc::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

/// Errors that can occur when dealing with the RTC.
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    /// Invalid input data was used (e.g. a year outside the 2000-2099 range).
    InvalidInputData,
}

/// Binary coded decimal with 2 bytes.
struct Bcd2 {
    pub tens: u8,
    pub units: u8,
}

/// Two 32-bit registers (RTC_TR and RTC_DR) contain the seconds, minutes, hours
/// (12- or 24-hour format), day (day of week), date (day of month), month, and
/// year, expressed in binary coded decimal format (BCD). The sub-seconds value
/// is also available in binary format.
///
/// The following helper functions encode into BCD format from integer and
/// decode to an integer from a BCD value respectively.
fn bcd2_encode(word: u32) -> Result<Bcd2, Error> {
    let tens = (word / 10)
        .try_into()
        .map_err(|_| Error::InvalidInputData)?;
    let units = (word % 10)
        .try_into()
        .map_err(|_| Error::InvalidInputData)?;
    Ok(Bcd2 { tens, units })
}

fn bcd2_decode(first: u8, second: u8) -> u32 {
    (first * 10 + second).into()
}

/// Entry point to the RTC API.
pub struct Rtc {
    rtc: pac::RTC,
    read_twice: bool,
}

impl Rtc {
    /// Initializes the RTC API.
    ///
    /// The `init` argument will only be used, if the real-time clock is not
    /// already configured. If the clock is not yet configured, and init is set
    /// to `None`, then the datetime corresponding to `2000-01-01 00:00:00`
    /// will be used for initialization.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidInputData`] if the `init` datetime is outside
    /// of the valid range (years 2000-2099).
    ///
    /// # Panics
    ///
    /// Panics, if the ABP1 clock frequency is lower than the RTC clock
    /// frequency. The RTC is currently hardcoded to use the LSE as clock source
    /// which runs at 32768 Hz.
    pub fn new(
        rtc: pac::RTC,
        rcc: &mut Rcc,
        pwr: &PWR,
        init: Option<NaiveDateTime>,
    ) -> Result<Self, Error> {
        // Backup write protection must be disabled by setting th DBP bit in
        // PWR_CR, otherwise it's not possible to access the RTC registers. We
        // assume that this was done during PWR initialization. To make sure it
        // already happened, this function requires a reference to PWR.

        // Select the LSE clock as the clock source for the RTC clock, as that's
        // the only source that keeps the RTC clocked and functional over system
        // resets.
        // Other clock sources are currently not supported.
        //
        // ATTENTION:
        // The prescaler settings in `set` assume that the LSE is selected, and
        // that the frequency is 32768 Hz. If you change the clock selection
        // here, you have to adapt the prescaler settings too.

        // Enable LSE clock
        rcc.enable_lse(pwr);
        rcc.rb.csr.modify(|_, w| {
            // Select LSE as RTC clock source.
            // This is safe, as we're writing a valid bit pattern.
            w.rtcsel().bits(0b01);
            // Enable RTC clock
            w.rtcen().set_bit()
        });

        let apb1_clk = rcc.clocks.apb1_clk();
        let rtc_clk = 32_768u32.Hz(); // LSE crystal frequency

        // The APB1 clock must not be slower than the RTC clock.
        if apb1_clk < rtc_clk {
            panic!(
                "APB1 clock ({}) is slower than RTC clock ({})",
                apb1_clk, rtc_clk,
            );
        }

        // If the APB1 clock frequency is less than 7 times the RTC clock
        // frequency, special care must be taken when reading some registers.
        let read_twice = apb1_clk.0 < 7 * rtc_clk.0;

        // Instantiate `Rtc` struct
        let mut rtc = Self { rtc, read_twice };

        // Initialize RTC, if not yet initialized
        if rtc.rtc.isr.read().inits().bit_is_clear() {
            rtc.set(init.unwrap_or_else(|| NaiveDate::from_ymd(2001, 1, 1).and_hms(0, 0, 0)))?;
        }

        // Disable wakeup timer. It's periodic and persists over resets, but for
        // ease of use, let's disable it on intialization, unless the user
        // wishes to start it again.
        //
        // Can't panic, as the error type is `Void`.
        rtc.wakeup_timer().cancel().unwrap();

        // Clear RSF bit, in case we woke up from Stop or Standby mode. This is
        // necessary, according to section 26.4.8.
        rtc.rtc.isr.write(|w| w.rsf().set_bit());

        Ok(rtc)
    }

    /// Sets the date/time.
    ///
    /// Note: Only dates in the range `2001-01-01 00:00:00` to
    /// `2099-12-31 23:59:59` are supported. If a date outside this range is
    /// passed in, [`Error::InvalidInputData`] will be returned.
    pub fn set(&mut self, instant: NaiveDateTime) -> Result<(), Error> {
        // Validate and encode datetime
        let y: i32 = instant.year();
        if !(2001..=2099).contains(&y) {
            return Err(Error::InvalidInputData);
        }
        let year = bcd2_encode((y - 2000) as u32)?;
        let month = bcd2_encode(instant.month())?;
        let day = bcd2_encode(instant.day())?;
        let hours = bcd2_encode(instant.hour())?;
        let minutes = bcd2_encode(instant.minute())?;
        let seconds = bcd2_encode(instant.second())?;

        // Write instant to RTC
        self.write(move |rtc| {
            // Start initialization
            rtc.isr.modify(|_, w| w.init().set_bit());

            // Wait until RTC register access is allowed
            while rtc.isr.read().initf().bit_is_clear() {}

            // Configure RTC. For now, the default values are all fine.
            rtc.cr.reset();

            // Configure the prescaler to generate a 1 Hz clock for the
            // calendar.
            //
            // ATTENTION:
            // This assumes the RTC clock frequency is 32768 Hz. If this
            // assumption holds no longer true, you need to change this code.
            rtc.prer.write(|w|
                // Safe, because we're only writing valid values to the fields.
                unsafe {
                    w.prediv_a().bits(0x7f);
                    w.prediv_s().bits(0xff)
                });

            // Write time
            rtc.tr.write(|w|
                // Safe, as `NaiveTime` verifies that its fields are valid.
                w
                    // 24-hour format
                    .pm().clear_bit()
                    // Hours
                    .ht().bits(hours.tens)
                    .hu().bits(hours.units)
                    // Minutes
                    .mnt().bits(minutes.tens)
                    .mnu().bits(minutes.units)
                    // Seconds
                    .st().bits(seconds.tens)
                    .su().bits(seconds.units));

            // Write date
            rtc.dr.write(|w|
                // Safe, as `NaiveDate` verifies that its fields are valid.
                w
                    // Year
                    .yt().bits(year.tens)
                    .yu().bits(year.units)
                    // Month
                    .mt().bit(month.tens > 0)
                    .mu().bits(month.units)
                    // Day
                    .dt().bits(day.tens)
                    .du().bits(day.units));

            // Exit initialization
            rtc.isr.modify(|_, w| w.init().clear_bit());
        });

        Ok(())
    }

    /// Read and return the current date/time from the RTC.
    pub fn now(&mut self) -> NaiveDateTime {
        // We need to wait until the RSF bit is set, for a multitude of reasons:
        // - In case the last read was within two cycles of RTCCLK. Not sure why
        //   that's important, but the documentation says so.
        // - In case the registers are not yet ready after waking up from a
        //   low-power mode.
        // - In case the registers are not yet ready after initialization.
        //
        // All of this is explain in section 26.4.8.
        while self.rtc.isr.read().rsf().bit_is_clear() {}

        // Reading the TR register locks the DR register until we clear the RSF
        // flag, so there's no danger of reading something weird here, as long
        // as this order of access is kept.
        let mut tr = self.rtc.tr.read();
        let mut dr = self.rtc.dr.read();

        // If the APB1 clock frequency is less than 7 times the RTC clock
        // frequency, we need to read the time and date registers twice, until
        // the two reads match. See section 26.4.8.
        if self.read_twice {
            loop {
                let tr2 = self.rtc.tr.read();
                let dr2 = self.rtc.dr.read();

                if tr.bits() == tr2.bits() && dr.bits() == dr2.bits() {
                    break;
                } else {
                    tr = tr2;
                    dr = dr2;
                }
            }
        }

        self.write(|rtc| {
            // Clear the RSF flag, to unlock the TR and DR registers.
            rtc.isr.write(|w| w.rsf().set_bit());
        });

        // Read date and time
        let year = bcd2_decode(dr.yt().bits(), dr.yu().bits()) as i32 + 2000;
        let month = bcd2_decode(dr.mt().bit() as u8, dr.mu().bits());
        let day = bcd2_decode(dr.dt().bits(), dr.du().bits());
        let hour = bcd2_decode(tr.ht().bits(), tr.hu().bits());
        let minute = bcd2_decode(tr.mnt().bits(), tr.mnu().bits());
        let second = bcd2_decode(tr.st().bits(), tr.su().bits());

        NaiveDate::from_ymd(year, month, day).and_hms(hour, minute, second)
    }

    /// Enable interrupts
    ///
    /// The interrupts set to `true` in `interrupts` will be enabled. Those set
    /// to false will not be modified.
    pub fn enable_interrupts(&mut self, interrupts: Interrupts) {
        self.write(|rtc| {
            rtc.cr.modify(|_, w| {
                if interrupts.timestamp {
                    w.tsie().set_bit();
                }
                if interrupts.wakeup_timer {
                    w.wutie().set_bit();
                }
                if interrupts.alarm_b {
                    w.alrbie().set_bit();
                }
                if interrupts.alarm_a {
                    w.alraie().set_bit();
                }
                w
            });
        })
    }

    /// Disable interrupts
    ///
    /// The interrupts set to `true` in `interrupts` will be disabled. Those set
    /// to false will not be modified.
    pub fn disable_interrupts(&mut self, interrupts: Interrupts) {
        self.write(|rtc| {
            rtc.cr.modify(|_, w| {
                if interrupts.timestamp {
                    w.tsie().clear_bit();
                }
                if interrupts.wakeup_timer {
                    w.wutie().clear_bit();
                }
                if interrupts.alarm_b {
                    w.alrbie().clear_bit();
                }
                if interrupts.alarm_a {
                    w.alraie().clear_bit();
                }
                w
            });
        })
    }

    /// Access the wakeup timer
    pub fn wakeup_timer(&mut self) -> WakeupTimer {
        WakeupTimer { rtc: self }
    }

    /// Disable write protection, run the passed in function, then re-enable
    /// write protection.
    fn write<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&pac::RTC) -> R,
    {
        // Disable write protection.
        // This is safe, as we're only writing the correct and expected values.
        self.rtc.wpr.write(|w| w.key().bits(0xca));
        self.rtc.wpr.write(|w| w.key().bits(0x53));

        let result = f(&self.rtc);

        // Re-enable write protection.
        // This is safe, as the field accepts the full range of 8-bit values.
        self.rtc.wpr.write(|w| w.key().bits(0xff));

        result
    }
}

/// Flags to enable/disable RTC interrupts.
#[derive(Default)]
pub struct Interrupts {
    pub timestamp: bool,
    pub wakeup_timer: bool,
    pub alarm_a: bool,
    pub alarm_b: bool,
}

/// The RTC wakeup timer
///
/// This timer can be used in two ways:
/// 1. Continually call `wait` until it returns `Ok(())`.
/// 2. Set up the RTC interrupt.
///
/// If you use an interrupt, you should still call `wait` once, after the
/// interrupt fired. This should return `Ok(())` immediately. Doing this will
/// reset the timer flag. If you don't do this, the interrupt will not fire
/// again, if you go to sleep.
///
/// You don't need to call `wait`, if you call `cancel`, as that also resets the
/// flag. Restarting the timer by calling `start` will also reset the flag.
pub struct WakeupTimer<'r> {
    rtc: &'r mut Rtc,
}

impl timer::Periodic for WakeupTimer<'_> {}

impl timer::CountDown for WakeupTimer<'_> {
    type Time = u32;

    /// Starts the wakeup timer
    ///
    /// The `delay` argument specifies the timer delay in seconds. Up to 17 bits
    /// of delay are supported, giving us a range of over 36 hours.
    ///
    /// # Panics
    ///
    /// The `delay` argument must be in the range `1 <= delay < 2^17`.
    /// Panics, if `delay` is outside of that range.
    fn start<T>(&mut self, delay: T)
    where
        T: Into<Self::Time>,
    {
        let delay = delay.into();
        assert!((1..=0x1_FF_FF).contains(&delay));

        let delay = delay - 1;

        // Can't panic, as the error type is `Void`.
        self.cancel().unwrap();

        self.rtc.write(|rtc| {
            // Set the wakeup delay
            rtc.wutr.write(|w|
                // Write the lower 16 bits of `delay`. The 17th bit is taken
                // care of via WUCKSEL in CR (see below).
                // This is safe, as the field accepts a full 16 bit value.
                w.wut().bits(delay as u16));
            // This is safe, as we're only writing valid bit patterns.
            rtc.cr.modify(|_, w| {
                if delay & 0x1_00_00 != 0 {
                    unsafe {
                        w.wucksel().bits(0b110);
                    }
                } else {
                    unsafe {
                        w.wucksel().bits(0b100);
                    }
                }

                // Enable wakeup timer
                w.wute().set_bit()
            });
        });

        // Let's wait for WUTWF to clear. Otherwise we might run into a race
        // condition, if the user calls this method again really quickly.
        while self.rtc.rtc.isr.read().wutwf().bit_is_set() {}
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.rtc.rtc.isr.read().wutf().bit_is_set() {
            self.rtc.write(|rtc| {
                // Clear wakeup timer flag
                rtc.isr.modify(|_, w| w.wutf().clear_bit());
            });

            return Ok(());
        }

        Err(nb::Error::WouldBlock)
    }
}

impl timer::Cancel for WakeupTimer<'_> {
    type Error = Void;

    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.rtc.write(|rtc| {
            // Disable the wakeup timer
            rtc.cr.modify(|_, w| w.wute().clear_bit());

            // Wait until we're allowed to update the wakeup timer configuration
            while rtc.isr.read().wutwf().bit_is_clear() {}

            // Clear wakeup timer flag
            rtc.isr.modify(|_, w| w.wutf().clear_bit());

            // According to the reference manual, section 26.7.4, the WUTF flag
            // must be cleared at least 1.5 RTCCLK periods "before WUTF is set
            // to 1 again". If that's true, we're on the safe side, because we
            // use ck_spre as the clock for this timer, which we've scaled to 1
            // Hz.
            //
            // I have the sneaking suspicion though that this is a typo, and the
            // quote in the previous paragraph actually tries to refer to WUTE
            // instead of WUTF. In that case, this might be a bug, so if you're
            // seeing something weird, adding a busy loop of some length here
            // would be a good start of your investigation.
        });

        Ok(())
    }
}
