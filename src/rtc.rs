//! Interface to the Real-time clock (RTC) peripheral
//!
//! See STM32L0x2 reference manual, chapter 26.


use void::Void;

use crate::{
    hal::timer::{
        self,
        Cancel as _,
    },
    pac,
    pwr::PWR,
    rcc::Rcc,
    time::U32Ext,
};


/// Entry point to the RTC API
pub struct RTC {
    rtc:        pac::RTC,
    read_twice: bool,
}

impl RTC {
    /// Initializes the RTC API
    ///
    /// The `initial_instant` argument will only be used, if the real-time clock
    /// is not already configured. Its `subsecond` field will be ignore in any
    /// case.
    ///
    /// # Panics
    ///
    /// Panics, if the ABP1 clock frequency is lower than the RTC clock
    /// frequency. The RTC is currently hardcoded to use the LSE as clock source
    /// which runs at 32768 Hz.
    pub fn new(
        rtc:  pac::RTC,
        rcc:  &mut Rcc,
        _:    &PWR,
        init: Instant,
    )
        -> Self
    {
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
        rcc.rb.csr.modify(|_, w| {
            // Select LSE as RTC clock source.
            // This is safe, as we're writing a valid bit pattern.
            #[cfg_attr(feature = "stm32l0x1", allow(unused_unsafe))]
            unsafe { w.rtcsel().bits(0b01); }

            w
                // Enable RTC clock
                .rtcen().set_bit()
                // Enable LSE clock
                .lseon().set_bit()
        });

        // Wait for LSE to be ready
        while rcc.rb.csr.read().lserdy().bit_is_clear() {}

        let apb1_clk = rcc.clocks.apb1_clk();
        let rtc_clk  = 32_768u32.hz(); // LSE crystal frequency

        // The APB1 clock must not be slower than the RTC clock.
        if apb1_clk < rtc_clk {
            panic!(
                "APB1 clock ({}) is slower than RTC clock ({})",
                apb1_clk,
                rtc_clk,
            );
        }

        // If the APB1 clock frequency is less than 7 times the RTC clock
        // frequency, special care must be taken when reading some registers.
        let read_twice = apb1_clk.0 < 7 * rtc_clk.0;

        let mut rtc = RTC {
            rtc,
            read_twice,
        };

        if rtc.rtc.isr.read().inits().bit_is_clear() {
            // RTC not yet initialized. Do that now.
            rtc.set(init);
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

        rtc
    }

    /// Sets the date/time
    pub fn set(&mut self, instant: Instant) {
        // Disable write protection.
        // This is safe, as we're only writin the correct and expected values.
        #[cfg_attr(feature = "stm32l0x1", allow(unused_unsafe))]
        self.rtc.wpr.write(|w| unsafe { w.key().bits(0xca) });
        #[cfg_attr(feature = "stm32l0x1", allow(unused_unsafe))]
        self.rtc.wpr.write(|w| unsafe { w.key().bits(0x53) });

        // Start initialization
        self.rtc.isr.modify(|_, w| w.init().set_bit());

        // Wait until RTC register access is allowed
        while self.rtc.isr.read().initf().bit_is_clear() {}

        // Configure RTC. For now, the default values are all fine.
        self.rtc.cr.reset();

        // Configure the prescaler to generate a 1 Hz clock for the calendar.
        //
        // ATTENTION:
        // This assumes the RTC clock frequency is 32768 Hz. If this assumption
        // holds no longer true, you need to change this code.
        self.rtc.prer.write(|w|
            // Safe, because we're only writing valid values to the fields.
            unsafe {
                w
                    .prediv_a().bits(0x7f)
                    .prediv_s().bits(0xff)
            }
        );

        // Write time
        #[cfg_attr(feature = "stm32l0x1", allow(unused_unsafe))]
        self.rtc.tr.write(|w|
            // Safe, as `Instant` verifies that its fields are valid.
            unsafe {
                w
                    // 24-hour format
                    .pm().clear_bit()
                    // Hour tens
                    .ht().bits(instant.hour / 10)
                    // Hour units
                    .hu().bits(instant.hour % 10)
                    // Minute tens
                    .mnt().bits(instant.minute / 10)
                    // Minute units
                    .mnu().bits(instant.minute % 10)
                    // Second tens
                    .st().bits(instant.second / 10)
                    // Second units
                    .su().bits(instant.second % 10)
            }
        );

        // Write date
        #[cfg_attr(feature = "stm32l0x1", allow(unused_unsafe))]
        self.rtc.dr.write(|w|
            // Safe, as `Instant` verifies that its fields are valid.
            unsafe {
                w
                    // Year tens
                    .yt().bits(instant.year / 10)
                    // Year units
                    .yu().bits(instant.year % 10)
                    // Month tens
                    .mt().bit(instant.month / 10 == 1)
                    // Month units
                    .mu().bits(instant.month % 10)
                    // Date tens
                    .dt().bits(instant.day / 10)
                    // Date units
                    .du().bits(instant.day % 10)
            }
        );

        // Exit initialization
        self.rtc.isr.modify(|_, w| w.init().clear_bit());
    }

    /// Returns the current date/time
    pub fn now(&mut self) -> Instant {
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
                }
                else {
                    tr = tr2;
                    dr = dr2;
                }
            }
        }

        // Clear the RSF flag, to unlock the TR and DR registers.
        self.rtc.isr.write(|w| w.rsf().set_bit());

        Instant {
            year:  dr.yt().bits()      * 10 + dr.yu().bits(),
            month: dr.mt().bit() as u8 * 10 + dr.mu().bits(),
            day:   dr.dt().bits()      * 10 + dr.du().bits(),

            hour:    tr.ht().bits() * 10 +  tr.hu().bits(),
            minute: tr.mnt().bits() * 10 + tr.mnu().bits(),
            second:  tr.st().bits() * 10 +  tr.su().bits(),
        }
    }

    /// Enable interrupts
    ///
    /// The interrupts set to `true` in `interrupts` will be enabled. Those set
    /// to false will not be modified.
    pub fn enable_interrupts(&mut self, interrupts: Interrupts) {
        self.rtc.cr.modify(|_, w| {
            if interrupts.timestamp { w.tsie().set_bit(); }
            if interrupts.wakeup_timer { w.wutie().set_bit(); }
            if interrupts.alarm_b { w.alrbie().set_bit(); }
            if interrupts.alarm_a { w.alraie().set_bit(); }
            w
        });
    }

    /// Disable interrupts
    ///
    /// The interrupts set to `true` in `interrupts` will be disabled. Those set
    /// to false will not be modified.
    pub fn disable_interrupts(&mut self, interrupts: Interrupts) {
        self.rtc.cr.modify(|_, w| {
            if interrupts.timestamp { w.tsie().clear_bit(); }
            if interrupts.wakeup_timer { w.wutie().clear_bit(); }
            if interrupts.alarm_b { w.alrbie().clear_bit(); }
            if interrupts.alarm_a { w.alraie().clear_bit(); }
            w
        });
    }

    /// Access the wakeup timer
    pub fn wakeup_timer(&mut self) -> WakeupTimer {
        WakeupTimer {
            rtc: &mut self.rtc,
        }
    }
}


/// An instant in time
///
/// You can create an instance of this struct using [`Instant::new`] or
/// [`RTC::now`].
#[derive(Clone, Copy, Debug)]
pub struct Instant {
    year:  u8,
    month: u8,
    day:   u8,

    hour:   u8,
    minute: u8,
    second: u8,
}

impl Instant {
    /// Creates a new `Instant`
    ///
    /// Initializes all fields with a default state, with `year`/`month`/`day`
    /// being `1` and `hour`/`minute`/`second` being `0`. You can use the
    /// various `set_*` methods to change the fields.
    ///
    /// Please note that all `set_*` methods validate their input, and will
    /// panic, if you pass an invalid value.
    ///
    /// Please also note, that the overall date is _not_ validated, so it's
    /// possible to create an `Instant` set to  February 31, for example.
    pub fn new() -> Self {
        Instant {
            year:  1,
            month: 1,
            day:   1,

            hour:   0,
            minute: 0,
            second: 0,
        }
    }

    /// Change the year
    ///
    /// # Panics
    ///
    /// Panics, if `year` is larger than `99`.
    pub fn set_year(mut self, year: u8) -> Self {
        assert!(year <= 99);
        self.year = year;
        self
    }

    /// Change the month
    ///
    /// # Panics
    ///
    /// Panics, if `month` is not a value from `1` to `12`.
    pub fn set_month(mut self, month: u8) -> Self {
        assert!(1 <= month && month <= 12);
        self.month = month;
        self
    }

    /// Change the day
    ///
    /// # Panics
    ///
    /// Panics, if `day` is not a value from `1` to `31`.
    pub fn set_day(mut self, day: u8) -> Self {
        assert!(1 <= day && day <= 31);
        self.day = day;
        self
    }

    /// Change the hour
    ///
    /// # Panics
    ///
    /// Panics, if `hour` is larger than `23`.
    pub fn set_hour(mut self, hour: u8) -> Self {
        assert!(hour <= 23);
        self.hour = hour;
        self
    }

    /// Change the minute
    ///
    /// # Panics
    ///
    /// Panics, if `minute` larger than `59`.
    pub fn set_minute(mut self, minute: u8) -> Self {
        assert!(minute <= 59);
        self.minute = minute;
        self
    }

    /// Change the second
    ///
    /// # Panics
    ///
    /// Panics, if `second` is larger than `59`.
    pub fn set_second(mut self, second: u8) -> Self {
        assert!(second <= 59);
        self.second = second;
        self
    }

    pub fn year(&self) -> u8 {
        self.year
    }

    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn day(&self) -> u8 {
        self.day
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    pub fn minute(&self) -> u8 {
        self.minute
    }

    pub fn second(&self) -> u8 {
        self.second
    }
}


pub struct Interrupts {
    pub timestamp:    bool,
    pub wakeup_timer: bool,
    pub alarm_a:      bool,
    pub alarm_b:      bool,
}

impl Default for Interrupts {
    fn default() -> Self {
        Self {
            timestamp:    false,
            wakeup_timer: false,
            alarm_a:      false,
            alarm_b:      false,
        }
    }
}


/// The RTC wakeup timer
pub struct WakeupTimer<'r> {
    rtc: &'r mut pac::RTC,
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
    /// The `delay` argument supports 17 bits. Panics, if a value larger than
    /// 17 bits is passed.
    fn start<T>(&mut self, delay: T)
        where T: Into<Self::Time>
    {
        let delay = delay.into();
        assert!(delay < 2^17);

        // Can't panic, as the error type is `Void`.
        self.cancel().unwrap();

        // Set the wakeup delay
        #[cfg_attr(feature = "stm32l0x1", allow(unused_unsafe))]
        self.rtc.wutr.write(|w|
            // Write the lower 16 bits of `delay`. The 17th bit is taken care of
            // via WUCKSEL in CR (see below).
            // This is safe, as the field accepts a full 16 bit value.
            unsafe { w.wut().bits(delay as u16) }
        );
        // This is safe, as we're only writing valid bit patterns.
        self.rtc.cr.modify(|_, w| {
            if delay & 0x1_00_00 != 0 {
                unsafe { w.wucksel().bits(0b110); }
            }
            else {
                unsafe { w.wucksel().bits(0b100); }
            }

            // Enable wakeup timer
            w.wute().set_bit()
        });

        // Let's wait for WUTFS to clear. Otherwise we might run into a race
        // condition, if the user calls this method again really quickly.
        while self.rtc.isr.read().wutwf().bit_is_set() {}
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.rtc.isr.read().wutf().bit_is_set() {
            return Ok(())
        }

        Err(nb::Error::WouldBlock)
    }
}

impl timer::Cancel for WakeupTimer<'_> {
    type Error = Void;

    fn cancel(&mut self) -> Result<(), Self::Error> {
        // Disable the wakeup timer
        self.rtc.cr.modify(|_, w| w.wute().clear_bit());

        // Wait until we're allowed to update the wakeup timer configuration
        while self.rtc.isr.read().wutwf().bit_is_clear() {}

        // Clear wakeup timer flag
        self.rtc.isr.modify(|_, w| w.wutf().clear_bit());

        // According to the reference manual, section 26.7.4, the WUTF flag must
        // be cleared at least 1.5 RTCCLK periods "before WUTF is set to 1
        // again". If that's true, we're on the safe side, because we use
        // ck_spre as the clock for this timer, which we've scaled to 1 Hz.
        //
        // I have the sneaking suspicion though that this is a typo, and the
        // quote in the previous paragraph actually tries to refer to WUTE
        // instead of WUTF. In that case, this might be a bug, so if you're
        // seeing something weird, adding a busy loop of some length here would
        // be a good start of your investigation.

        Ok(())
    }
}
