use core::marker::PhantomData;
use core::mem;
use core::ops::Deref;

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


pub struct Timer<I, P: Pins<I>> {
    _instance: I,

    pub channels: P::Channels,
}

impl<I, P> Timer<I, P>
    where
        I: Instance,
        P: Pins<I>,
{
    pub fn new(timer: I, pins: P, frequency: Hertz, rcc: &mut Rcc) -> Self {
        pins.setup();
        timer.enable(rcc);

        let clk = timer.clock_frequency(rcc);
        let freq = frequency.0;
        let ticks = clk / freq;
        let psc = u16((ticks - 1) / (1 << 16)).unwrap();
        let arr = u16(ticks / u32(psc + 1)).unwrap();
        timer.psc.write(|w| unsafe { w.psc().bits(psc) });
        timer.arr.write(|w| w.arr().bits(arr.into()));
        timer.cr1.write(|w| w.cen().set_bit());

        Self {
            _instance: timer,
            // I'm not sure about this `unsafe`. It should be fine, as
            // `channels` should have no data that is ever accessed, but it
            // seems fishy, and there's probably a more elegant solution.
            channels: unsafe { mem::uninitialized() },
        }
    }
}


pub trait Instance: Deref<Target=tim2::RegisterBlock> {
    fn ptr() -> *const tim2::RegisterBlock;
    fn enable(&self, _: &mut Rcc);
    fn clock_frequency(&self, _: &mut Rcc) -> u32;
}

macro_rules! impl_instance {
    (
        $(
            $name:ty,
            $apbXenr:ident,
            $apbXrstr:ident,
            $timXen:ident,
            $timXrst:ident,
            $apbX_clk:ident;
        )*
    ) => {
        $(
            impl Instance for $name {
                fn ptr() -> *const tim2::RegisterBlock {
                    Self::ptr()
                }

                fn enable(&self, rcc: &mut Rcc) {
                    rcc.rb.$apbXenr.modify(|_, w| w.$timXen().set_bit());
                    rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().set_bit());
                    rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().clear_bit());
                }

                fn clock_frequency(&self, rcc: &mut Rcc) -> u32 {
                    rcc.clocks.$apbX_clk().0
                }
            }
        )*
    }
}

impl_instance!(
    TIM2, apb1enr, apb1rstr, tim2en, tim2rst, apb1_clk;
);


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


pub struct Pwm<TIM, CHANNEL> {
    _channel: PhantomData<CHANNEL>,
    _tim: PhantomData<TIM>,
}

impl<I, C> hal::PwmPin for Pwm<I, C>
    where
        I: Instance,
        C: Channel,
{
    type Duty = u16;

    fn disable(&mut self) {
        // This is UNSAFE: Race condition during read-modify-write.
        C::disable(unsafe { &*I::ptr() });
    }

    fn enable(&mut self) {
        // This is UNSAFE: Race condition during read-modify-write.
        C::enable(unsafe { &*I::ptr() });
    }

    fn get_duty(&self) -> u16 {
        // Safe, as we're only doing an atomic read.
        C::get_duty(unsafe { &*I::ptr() })
    }

    fn get_max_duty(&self) -> u16 {
        // Safe, as we're only doing an atomic read.
        let tim = unsafe { &*I::ptr() };

        // This cast to `u16` is fine. The type is already `u16`, but on
        // STM32L0x2, the SVD file seems to be wrong about that (or the
        // reference manual is wrong; but in any case, we only ever write `u16`
        // into this field).
        tim.arr.read().arr().bits() as u16
    }

    fn set_duty(&mut self, duty: u16) {
        // Safe, as we're only doing an atomic write.
        C::set_duty(unsafe { &*I::ptr() }, duty);
    }
}


macro_rules! channels {
    ($TIMX:ident, $af:expr, $c1:ty, $c2:ty, $c3:ty, $c4:ty) => {
        impl Pins<$TIMX> for $c1 {
            type Channels = Pwm<$TIMX, C1>;

            fn setup(&self) {
                self.set_alt_mode($af);
            }
        }

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

channels!(
    TIM2,
    AltMode::AF2,
    PA0<Input<Floating>>,
    PA1<Input<Floating>>,
    PA2<Input<Floating>>,
    PA3<Input<Floating>>
);
