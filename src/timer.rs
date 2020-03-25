//! Timers
use crate::hal::timer::{CountDown, Periodic};
use crate::pac::{TIM2, TIM21, TIM22, TIM3, tim2, tim21, tim22};
use crate::rcc::{Clocks, Rcc};
use crate::time::Hertz;
use cast::{u16, u32};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SYST;
use nb;
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

macro_rules! timers {
    ($($TIM:ident: ($tim:ident, $timXen:ident, $timXrst:ident, $apbenr:ident, $apbrstr:ident, $timclk:ident, $mms:ty),)+) => {
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
                    rcc.rb.$apbenr.modify(|_, w| w.$timXen().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().clear_bit());

                    let mut timer = Timer {
                        tim,
                        clocks: rcc.clocks,
                    };
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

timers! {
    TIM2: (tim2, tim2en, tim2rst, apb1enr, apb1rstr, apb1_tim_clk,
        tim2::cr2::MMS_A),
    TIM3: (tim3, tim3en, tim3rst, apb1enr, apb1rstr, apb1_tim_clk,
        tim2::cr2::MMS_A),
    TIM21: (tim21, tim21en, tim21rst, apb2enr, apb2rstr, apb2_tim_clk,
        tim21::cr2::MMS_A),
    TIM22: (tim22, tim22en, tim22rst, apb2enr, apb2rstr, apb2_tim_clk,
        tim22::cr2::MMS_A),
}


pub trait GeneralPurposeTimer {
    type MasterMode;

    fn select_master_mode(&mut self, variant: Self::MasterMode);
}
