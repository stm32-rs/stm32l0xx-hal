use core::marker::PhantomData;
use core::mem;

use crate::gpio::gpioa::{PA0, PA1, PA2, PA3};
use crate::gpio::{AltMode, Floating, Input};
use crate::hal;
use crate::pac::{
    tim2,
    TIM2,
};
use crate::rcc::Rcc;
use crate::time::Hertz;
use cast::{u16, u32};


pub trait Channel {
    fn disable(_: &tim2::RegisterBlock);
    fn enable(_: &tim2::RegisterBlock);
    fn get_duty(_: &tim2::RegisterBlock) -> u16;
    fn set_duty(_: &tim2::RegisterBlock, duty: u16);
}

macro_rules! impl_channel {
    (
        $(
            $name:ident,
            $ccxe:ident,
            $ccmr_output:ident,
            $ocxpe:ident,
            $ocxm:ident,
            $ccrx:ident;
        )*
    ) => {
        $(
            pub struct $name;

            impl Channel for $name {
                fn disable(tim: &tim2::RegisterBlock) {
                    tim.ccer.modify(|_, w| w.$ccxe().clear_bit());
                }

                fn enable(tim: &tim2::RegisterBlock) {
                    tim.$ccmr_output.modify(|_, w| {
                        w.$ocxpe().set_bit();
                        // Safe. We're writing a valid bit pattern.
                        unsafe { w.$ocxm().bits(0b110) }
                    });
                    tim.ccer.modify(|_, w| w.$ccxe().set_bit());
                }

                fn get_duty(tim: &tim2::RegisterBlock) -> u16 {
                    // This cast to `u16` is fine. The type is already `u16`,
                    // but on STM32L0x2, the SVD file seems to be wrong about
                    // that (or the reference manual is wrong; but in any case,
                    // we only ever write `u16` into this field).
                    tim.$ccrx.read().ccr().bits() as u16
                }

                fn set_duty(tim: &tim2::RegisterBlock, duty: u16) {
                    tim.$ccrx.write(|w| w.ccr().bits(duty.into()));
                }
            }
        )*
    }
}

impl_channel!(
    C1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1;
    C2, cc2e, ccmr1_output, oc2pe, oc2m, ccr2;
    C3, cc3e, ccmr2_output, oc3pe, oc3m, ccr3;
    C4, cc4e, ccmr2_output, oc4pe, oc4m, ccr4;
);


pub trait Pins<TIM> {
    type Channels;
    fn setup(&self);
}

pub trait PwmExt: Sized {
    fn pwm<PINS, T>(self, _: PINS, frequency: T, rcc: &mut Rcc) -> PINS::Channels
    where
        PINS: Pins<Self>,
        T: Into<Hertz>;
}

pub struct Pwm<TIM, CHANNEL> {
    _channel: PhantomData<CHANNEL>,
    _tim: PhantomData<TIM>,
}

macro_rules! channels {
    ($TIMX:ident, $af:expr, $c1:ty) => {
        impl Pins<$TIMX> for $c1 {
            type Channels = Pwm<$TIMX, C1>;

            fn setup(&self) {
                self.set_alt_mode($af);
            }
        }

        impl<C> hal::PwmPin for Pwm<$TIMX, C>
            where C: Channel
        {
            type Duty = u16;

            fn disable(&mut self) {
                // This is UNSAFE: Race condition during read-modify-write.
                C::disable(unsafe { &*$TIMX::ptr() });
            }

            fn enable(&mut self) {
                // This is UNSAFE: Race condition during read-modify-write.
                C::enable(unsafe { &*$TIMX::ptr() });
            }

            fn get_duty(&self) -> u16 {
                // Safe, as we're only doing an atomic read.
                C::get_duty(unsafe { &*$TIMX::ptr() })
            }

            fn get_max_duty(&self) -> u16 {
                // Safe, as we're only doing an atomic read.
                let tim = unsafe { &*$TIMX::ptr() };

                // This cast to `u16` is fine. The type is already `u16`, but on
                // STM32L0x2, the SVD file seems to be wrong about that (or the
                // reference manual is wrong; but in any case, we only ever
                // write `u16` into this field).
                tim.arr.read().arr().bits() as u16
            }

            fn set_duty(&mut self, duty: u16) {
                // Safe, as we're only doing an atomic write.
                C::set_duty(unsafe { &*$TIMX::ptr() }, duty);
            }
        }
    };
    ($TIMX:ident, $af:expr, $c1:ty, $c2:ty, $c3:ty, $c4:ty) => {
        channels!($TIMX, $af, $c1);

        impl Pins<$TIMX> for $c2 {
            type Channels = Pwm<$TIMX, C2>;

            fn setup(&self) {
                self.set_alt_mode($af);
            }
        }

        impl Pins<$TIMX> for $c3 {
            type Channels = Pwm<$TIMX, C3>;

            fn setup(&self) {
                self.set_alt_mode($af);
            }
        }

        impl Pins<$TIMX> for $c4 {
            type Channels = Pwm<$TIMX, C4>;

            fn setup(&self) {
                self.set_alt_mode($af);
            }
        }

        impl Pins<$TIMX> for ($c1, $c2) {
            type Channels = (Pwm<$TIMX, C1>, Pwm<$TIMX, C2>);

            fn setup(&self) {
                self.0.set_alt_mode($af);
                self.1.set_alt_mode($af);
            }
        }

        impl Pins<$TIMX> for ($c1, $c2, $c3, $c4) {
            type Channels = (
                Pwm<$TIMX, C1>,
                Pwm<$TIMX, C2>,
                Pwm<$TIMX, C3>,
                Pwm<$TIMX, C4>,
            );

            fn setup(&self) {
                self.0.set_alt_mode($af);
                self.1.set_alt_mode($af);
                self.2.set_alt_mode($af);
                self.3.set_alt_mode($af);
            }
        }
    };
}

macro_rules! timers {
    ($($TIMX:ident: ($apb_clk:ident, $apbXenr:ident, $apbXrstr:ident, $timX:ident, $timXen:ident, $timXrst:ident),)+) => {
        $(
            impl PwmExt for $TIMX {
                fn pwm<PINS, T>(
                    self,
                    pins: PINS,
                    freq: T,
                    rcc: &mut Rcc,
                ) -> PINS::Channels
                where
                    PINS: Pins<Self>,
                    T: Into<Hertz>,
                {
                    $timX(self, pins, freq.into(), rcc)
                }
            }

            fn $timX<PINS>(
                tim: $TIMX,
                pins: PINS,
                freq: Hertz,
                rcc: &mut Rcc,
            ) -> PINS::Channels
            where
                PINS: Pins<$TIMX>,
            {
                pins.setup();
                rcc.rb.$apbXenr.modify(|_, w| w.$timXen().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().clear_bit());

                let clk = rcc.clocks.$apb_clk().0;
                let freq = freq.0;
                let ticks = clk / freq;
                let psc = u16((ticks - 1) / (1 << 16)).unwrap();
                let arr = u16(ticks / u32(psc + 1)).unwrap();
                tim.psc.write(|w| unsafe { w.psc().bits(psc) });
                #[allow(unused_unsafe)]
                #[cfg(feature = "stm32l0x1")]
                tim.arr.write(|w| unsafe { w.arr().bits(arr) });
                #[cfg(feature = "stm32l0x2")]
                tim.arr.write(|w| w.arr().bits(arr as u32));
                tim.cr1.write(|w| w.cen().set_bit());
                unsafe { mem::uninitialized() }
            }
        )+
    }
}

channels!(
    TIM2,
    AltMode::AF2,
    PA0<Input<Floating>>,
    PA1<Input<Floating>>,
    PA2<Input<Floating>>,
    PA3<Input<Floating>>
);

timers! {
    TIM2: (apb1_clk, apb1enr, apb1rstr, tim2, tim2en, tim2rst),
}
