//! Encoder input using timers.

use crate::gpio::{gpioa, gpiob, AltMode, Analog};
use crate::pac::{tim2, tim21, TIM2, TIM21};
use crate::rcc::{Enable, Rcc, Reset};
use core::marker::PhantomData;

pub trait Pins<TIM> {
    fn into_alt_mode(self);
}

pub trait PinCh1<TIM> {
    fn into_alt_mode(self);
}
pub trait PinCh2<TIM> {
    fn into_alt_mode(self);
}

impl PinCh1<TIM21> for gpiob::PB13<Analog> {
    fn into_alt_mode(self) {
        self.set_alt_mode(AltMode::AF6);
    }
}
impl PinCh2<TIM21> for gpiob::PB14<Analog> {
    fn into_alt_mode(self) {
        self.set_alt_mode(AltMode::AF6);
    }
}

impl PinCh1<TIM2> for gpioa::PA0<Analog> {
    fn into_alt_mode(self) {
        self.set_alt_mode(AltMode::AF2);
    }
}
impl PinCh2<TIM2> for gpioa::PA1<Analog> {
    fn into_alt_mode(self) {
        self.set_alt_mode(AltMode::AF2);
    }
}

impl<TIM, CH1, CH2> Pins<TIM> for (CH1, CH2)
where
    CH1: PinCh1<TIM>,
    CH2: PinCh2<TIM>,
{
    fn into_alt_mode(self) {
        self.0.into_alt_mode();
        self.1.into_alt_mode();
    }
}

/// Encoder direction.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Direction {
    /// Encoder is counting up.
    Up,
    /// Encoder is counting down.
    Down,
}

/// Encoder status.
pub struct Status {
    /// Encoder direction.
    pub direction: Direction,
    /// Overflow flag.
    pub did_overflow: bool,
    /// Current encoder count.
    pub count: u16,
}

/// Encoder mode. See `TIMx_SMCR.SMS` item in datasheet.
pub enum Mode {
    /// Encoder mode 1 - Counter counts up/down on TI2FP1 edge depending on TI1FP2 level.
    CountTi2 = 0b001,
    /// Encoder mode 2 - Counter counts up/down on TI1FP2 edge depending on TI2FP1 level.
    CountTi1 = 0b010,
    ///  Encoder mode 3 (AKA quadrature mode) - Counter counts up/down on both TI1FP1 and TI2FP2 edges depending on the level of the other input.
    Qei = 0b011,
}

pub trait EncoderExt<TIM> {
    fn encoder<PINS>(self, pins: PINS, mode: Mode, arr: u16, rcc: &mut Rcc) -> Encoder<TIM, PINS>
    where
        PINS: Pins<TIM>;
}

pub struct Encoder<T, PINS> {
    timer: T,
    _pins: PhantomData<PINS>,
}

macro_rules! encoders {
    ($($TIM:ident: ($sms:ty),)+) => {
        $(
            impl EncoderExt<$TIM> for $TIM {
                fn encoder<PINS>(
                    self,
                    pins: PINS,
                    mode: Mode,
                    arr: u16,
                    rcc: &mut Rcc,
                ) -> Encoder<$TIM, PINS>
                where
                    PINS: Pins<$TIM>,
                {
                    Encoder::<$TIM, PINS>::new(self, pins, mode, arr, rcc)
                }
            }

            impl<PINS> Encoder<$TIM, PINS>
            where
                PINS: Pins<$TIM>,
            {
                fn new(timer: $TIM, pins: PINS, mode: Mode, arr: u16, rcc: &mut Rcc) -> Self {
                    // Enable peripheral, reset it
                    <$TIM>::enable(rcc);
                    <$TIM>::reset(rcc);

                    // Disable the timer for configuration
                    timer.cr1.write(|w| w.cen().clear_bit());

                    // Configure encoder inputs.
                    pins.into_alt_mode();

                    // Encoder mode, count on all edges
                    timer.smcr.write(|w| {
                        w
                            // Trigger source - TI1 edge detector
                            .ts()
                            .ti1f_ed()
                            // Count edges on TI1, direction set by TI2
                            .sms()
                            .variant(match mode {
                                Mode::CountTi1 => <$sms>::EncoderMode1,
                                Mode::CountTi2 => <$sms>::EncoderMode2,
                                Mode::Qei => <$sms>::EncoderMode3,
                            })
                    });

                    timer.cr1.write(|w| {
                        w
                            // Only interrupt on over/underflow (as well as input pulses)
                            .urs()
                            .set_bit()
                            // Enable timer
                            .cen()
                            .set_bit()
                    });

                    // "After setting the ENABLE bit, a delay of two counter clock is needed before the LPTIM is
                    // actually enabled."
                    // The slowest LPTIM clock source is LSE at 32768 Hz, the fastest CPU clock is ~80 MHz. At
                    // these conditions, one cycle of the LPTIM clock takes 2500 CPU cycles, so sleep for 5000.
                    cortex_m::asm::delay(5000);

                    let mut self_ = Self {
                        timer,
                        _pins: PhantomData,
                    };

                    self_.set_arr(arr);

                    self_
                }

                /// Get the ARR (Auto Reload Register) value.
                pub fn arr(&mut self) -> u16 {
                    self.timer.arr.read().bits() as u16
                }

                /// Set ARR (Auto Reload Register) value.
                ///
                /// ARR may only be set when timer is enabled.
                pub fn set_arr(&mut self, arr: u16) {
                    // This is only unsafe for some timers, so we need this to suppress the
                    // warnings.
                    #[allow(unused_unsafe)]
                    self.timer.arr.write(|w| unsafe { w.arr().bits(arr) });
                }

                /// Listen for over/underflow interrupts
                pub fn listen(&mut self) {
                    // Listen for over/underflow.
                    self.timer.dier.write(|w| w.uie().enabled());
                }

                /// Listen for over/underflow interrupts, as well as IO triggers.
                pub fn listen_all(&mut self) {
                    // Listen for over/underflow.
                    self.listen();

                    // Listen for counter change as well.
                    self.timer.dier.write(|w| w.tie().enabled());
                }

                /// Get the current encoder status.
                ///
                /// Note that calling `clear_irq` before this method will clear the `did_overflow` flag.
                pub fn status(&mut self) -> Status {
                    // Up = 0, down = 1
                    let is_dir_up = self.timer.cr1.read().dir().bit_is_clear();

                    // UIF is bit 0
                    let did_overflow = self.timer.sr.read().bits() & 0x01 == 1;

                    let count = self.timer.cnt.read().bits() as u16;

                    let direction = match is_dir_up {
                        true => Direction::Up,
                        false => Direction::Down,
                    };

                    Status {
                        direction,
                        did_overflow,
                        count,
                    }
                }

                /// Clear all interrupts
                pub fn clear_irq(&mut self) {
                    self.timer.sr.reset();
                }
            }
        )+
    }
}

encoders! {
    TIM2: (tim2::smcr::SMS_A),
    TIM21: (tim21::smcr::SMS_A),
}
