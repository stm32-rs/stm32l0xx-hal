//! Timers
use crate::hal::timer::{CountDown, Periodic};
use crate::pac::{tim2, tim21, tim22, tim6, TIM2, TIM21, TIM22, TIM3, TIM6};
use crate::rcc::{Clocks, Enable, Rcc, Reset};
use cast::{u16, u32};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SYST;
use embedded_time::rate::Hertz;
use void::Void;

pub trait TimerExt<TIM> {
    fn timer<T>(self, timeout: T, rcc: &mut Rcc) -> Timer<TIM>
    where
        T: Into<Hertz>;
}

/// Hardware timers
pub struct Timer<TIM> {
    clocks: Clocks,
    tim: TIM,
}

impl Timer<SYST> {
    /// Configures the SYST clock as a periodic count down timer
    pub fn syst<T>(mut syst: SYST, timeout: T, rcc: &mut Rcc) -> Self
    where
        T: Into<Hertz>,
    {
        syst.set_clock_source(SystClkSource::Core);
        let mut timer = Timer {
            tim: syst,
            clocks: rcc.clocks,
        };
        timer.start(timeout);
        timer
    }

    /// Starts listening
    pub fn listen(&mut self) {
        self.tim.enable_interrupt()
    }

    /// Stops listening
    pub fn unlisten(&mut self) {
        self.tim.disable_interrupt()
    }
}

impl CountDown for Timer<SYST> {
    type Time = Hertz;

    fn start<T>(&mut self, timeout: T)
    where
        T: Into<Hertz>,
    {
        let rvr = self.clocks.sys_clk().0 / timeout.into().0 - 1;
        assert!(rvr < (1 << 24));

        self.tim.set_reload(rvr);
        self.tim.clear_current();
        self.tim.enable_counter();
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.tim.has_wrapped() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl TimerExt<SYST> for SYST {
    fn timer<T>(self, timeout: T, rcc: &mut Rcc) -> Timer<SYST>
    where
        T: Into<Hertz>,
    {
        Timer::syst(self, timeout, rcc)
    }
}

impl Periodic for Timer<SYST> {}

/// Trait for general purpose timer peripherals
pub trait GeneralPurposeTimer: Enable + Reset {
    type MasterMode;

    /// Selects the master mode for this timer
    fn select_master_mode(&mut self, variant: Self::MasterMode);
}

impl<T: GeneralPurposeTimer> Timer<T> {
    pub fn new(tim: T, rcc: &mut Rcc) -> Self {
        T::enable(rcc);
        T::reset(rcc);
        Timer {
            tim,
            clocks: rcc.clocks,
        }
    }
}

macro_rules! timers {
    ($($TIM:ident: ($tim:ident, $timclk:ident, $mms:ty),)+) => {
        $(
            impl TimerExt<$TIM> for $TIM {
                fn timer<T>(self, timeout: T, rcc: &mut Rcc) -> Timer<$TIM>
                    where
                        T: Into<Hertz>,
                {
                    Timer::$tim(self, timeout, rcc)
                }
            }

            impl Timer<$TIM> where $TIM: GeneralPurposeTimer {
                /// Configures a TIM peripheral as a periodic count down timer
                pub fn $tim<T>(tim: $TIM, timeout: T, rcc: &mut Rcc) -> Self
                where
                    T: Into<Hertz>,
                {
                    let mut timer = Timer::new(tim, rcc);
                    timer.start(timeout);
                    timer
                }

                /// Starts listening
                pub fn listen(&mut self) {
                    self.tim.dier.write(|w| w.uie().set_bit());
                }

                /// Stops listening
                pub fn unlisten(&mut self) {
                    self.tim.dier.write(|w| w.uie().clear_bit());
                }

                /// Clears interrupt flag
                pub fn clear_irq(&mut self) {
                    self.tim.sr.write(|w| w.uif().clear_bit());
                }

                /// Releases the TIM peripheral
                pub fn release(self) -> $TIM {
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                    self.tim
                }

                /// Reset counter
                pub fn reset(&mut self) {
                    // pause
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                    // reset counter
                    self.tim.cnt.reset();
                    // continue
                    self.tim.cr1.modify(|_, w| w.cen().set_bit());
                }

                /// Select master mode
                pub fn select_master_mode(&mut self,
                    variant: <$TIM as GeneralPurposeTimer>::MasterMode,
                ) {
                    self.tim.select_master_mode(variant);
                }
            }

            impl CountDown for Timer<$TIM> {
                type Time = Hertz;

                fn start<T>(&mut self, timeout: T)
                where
                    T: Into<Hertz>,
                {
                    // pause
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                    // reset counter
                    self.tim.cnt.reset();

                    let freq = timeout.into().0;
                    let ticks = self.clocks.$timclk().0 / freq;
                    let psc = u16((ticks - 1) / (1 << 16)).unwrap();
                    self.tim.psc.write(|w| w.psc().bits(psc));
                    // This is only unsafe for some timers, so we need this to
                    // suppress the warnings.
                    #[allow(unused_unsafe)]
                    self.tim.arr.write(|w|
                        unsafe {
                            w.arr().bits(u16(ticks / u32(psc + 1)).unwrap())
                        }
                    );

                    // Load prescaler value and reset its counter.
                    // Setting URS makes sure no interrupt is generated.
                    self.tim.cr1.modify(|_, w| w.urs().set_bit());
                    self.tim.egr.write(|w| w.ug().set_bit());

                    self.tim.cr1.modify(|_, w| w.cen().set_bit());
                }

                fn wait(&mut self) -> nb::Result<(), Void> {
                    if self.tim.sr.read().uif().bit_is_clear() {
                        Err(nb::Error::WouldBlock)
                    } else {
                        self.tim.sr.modify(|_, w| w.uif().clear_bit());
                        Ok(())
                    }
                }
            }

            impl Periodic for Timer<$TIM> {}

            impl GeneralPurposeTimer for $TIM {
                type MasterMode = $mms;

                fn select_master_mode(&mut self, variant: Self::MasterMode) {
                    self.cr2.modify(|_, w| w.mms().variant(variant));
                }
            }
        )+
    }
}

/// Two linked 16 bit timers that form a 32 bit timer.
pub trait LinkedTimer {
    /// Return the current 16 bit counter value of the MSB timer.
    fn get_counter_msb(&self) -> u16;

    /// Return the current 16 bit counter value of the LSB timer.
    fn get_counter_lsb(&self) -> u16;

    /// Return the current 32 bit counter value.
    fn get_counter(&self) -> u32;

    /// Reset the counter to 0.
    fn reset(&mut self);
}

/// A pair of timers that can be linked.
///
/// The two timers are configured so that an overflow of the primary timer
/// triggers an update on the secondary timer. This way, two 16 bit timers can
/// be combined to a single 32 bit timer.
pub struct LinkedTimerPair<PRIMARY, SECONDARY> {
    /// Timer in primary mode
    tim_primary: PRIMARY,
    /// Timer in secondary mode
    tim_secondary: SECONDARY,
}

macro_rules! linked_timers {
    ($(
        ($PRIMARY:ident, $SECONDARY:ident): (
            $new:ident,
            $mms:ty, $sms:ty, $ts:expr
        ),
    )+) => {
        $(
            impl LinkedTimerPair<$PRIMARY, $SECONDARY> {
                /// Create and configure a new `LinkedTimerPair` with the
                /// specified timers.
                pub fn $new(tim_primary: $PRIMARY, tim_secondary: $SECONDARY, rcc: &mut Rcc) -> Self {
                    // Enable timers
                    <$PRIMARY>::enable(rcc);
                    <$SECONDARY>::enable(rcc);

                    // Reset timers
                    <$PRIMARY>::reset(rcc);
                    <$SECONDARY>::reset(rcc);

                    // Enable counter
                    tim_primary.cr1.modify(|_, w| w.cen().set_bit());
                    tim_secondary.cr1.modify(|_, w| w.cen().set_bit());

                    // In the MMS (Master Mode Selection) register, set the master mode so
                    // that a rising edge is output on the trigger output TRGO every time
                    // an update event is generated.
                    tim_primary.cr2.modify(|_, w| w.mms().variant(<$mms>::UPDATE));

                    // In the SMCR (Slave Mode Control Register), select the
                    // appropriate internal trigger source (TS).
                    tim_secondary.smcr.modify(|_, w| w.ts().variant($ts));
                    // Set the SMS (Slave Mode Selection) register to "external clock mode 1",
                    // where the rising edges of the selected trigger (TRGI) clock the counter.
                    tim_secondary.smcr.modify(|_, w| w.sms().variant(<$sms>::EXT_CLOCK_MODE));

                    Self { tim_primary, tim_secondary }
                }
            }

            impl LinkedTimer for LinkedTimerPair<$PRIMARY, $SECONDARY> {
                /// Return the current 16 bit counter value of the primary timer (LSB).
                fn get_counter_lsb(&self) -> u16 {
                    self.tim_primary.cnt.read().cnt().bits()
                }

                /// Return the current 16 bit counter value of the secondary timer (MSB).
                fn get_counter_msb(&self) -> u16 {
                    self.tim_secondary.cnt.read().cnt().bits()
                }

                /// Return the current 32 bit counter value.
                ///
                /// Note: Due to the potential for a race condition between
                /// reading MSB and LSB, it's possible that the registers must
                /// be re-read once. Therefore reading the counter value is not
                /// constant time.
                fn get_counter(&self) -> u32 {
                    loop {
                        let msb = self.tim_secondary.cnt.read().cnt().bits() as u32;
                        let lsb = self.tim_primary.cnt.read().cnt().bits() as u32;

                        // Because the timer is still running at high frequency
                        // between reading MSB and LSB, it's possible that LSB
                        // has already overflowed. Therefore we read MSB again
                        // to check that it hasn't changed.
                        let msb_again = self.tim_secondary.cnt.read().cnt().bits() as u32;
                        if msb == msb_again {
                            return (msb << 16) | lsb;
                        }
                    }
                }

                fn reset(&mut self) {
                    // Pause
                    self.tim_primary.cr1.modify(|_, w| w.cen().clear_bit());
                    self.tim_secondary.cr1.modify(|_, w| w.cen().clear_bit());
                    // Reset counter
                    self.tim_primary.cnt.reset();
                    self.tim_secondary.cnt.reset();
                    // Continue
                    self.tim_secondary.cr1.modify(|_, w| w.cen().set_bit());
                    self.tim_primary.cr1.modify(|_, w| w.cen().set_bit());
                }
            }
        )+
    }
}

timers! {
    TIM2: (tim2, apb1_tim_clk, tim2::cr2::MMS_A),
    TIM3: (tim3, apb1_tim_clk, tim2::cr2::MMS_A),
    TIM6: (tim6, apb1_tim_clk, tim6::cr2::MMS_A),
    TIM21: (tim21, apb2_tim_clk, tim21::cr2::MMS_A),
    TIM22: (tim22, apb2_tim_clk, tim22::cr2::MMS_A),
}

linked_timers! {
    // Internal trigger connection: RM0377 table 76
    (TIM2, TIM3): (tim2_tim3, tim2::cr2::MMS_A, tim2::smcr::SMS_A, tim2::smcr::TS_A::ITR0),
    // Internal trigger connection: RM0377 table 80
    (TIM21, TIM22): (tim21_tim22, tim21::cr2::MMS_A, tim22::smcr::SMS_A, tim22::smcr::TS_A::ITR0),

    // Note: Other combinations would be possible as well, e.g. (TIM21, TIM2) or (TIM2, TIM22).
    // They can be implemented if needed.
}
