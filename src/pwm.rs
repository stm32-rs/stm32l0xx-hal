use crate::gpio::gpioa::{PA0, PA1, PA2, PA3};
use crate::gpio::{AltMode, PinMode};
use crate::hal;
use crate::pac::{tim2, TIM2, TIM3};
use crate::rcc::{Enable, Rcc, Reset};
use cast::{u16, u32};
use core::marker::PhantomData;
use core::ops::Deref;
use cortex_m::interrupt;
use embedded_time::rate::Hertz;

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
use crate::gpio::{
    gpioa::{PA15, PA5},
    gpiob::{PB10, PB11, PB3},
};

#[cfg(any(feature = "stm32l072", feature = "stm32l082", feature = "io-STM32L071",))]
use crate::gpio::{
    gpioa::{PA6, PA7},
    gpiob::{PB0, PB1, PB4, PB5},
};

#[cfg(feature = "stm32l072")]
use crate::gpio::{
    gpioc::{PC6, PC7, PC8, PC9},
    gpioe::{PE10, PE11, PE12, PE3, PE4, PE5, PE6, PE9},
};

pub struct Timer<I> {
    instance: I,

    pub channel1: Pwm<I, C1, Unassigned>,
    pub channel2: Pwm<I, C2, Unassigned>,
    pub channel3: Pwm<I, C3, Unassigned>,
    pub channel4: Pwm<I, C4, Unassigned>,
}

impl<I> Timer<I>
where
    I: Instance,
{
    /// Create new timer instance that is automatically started with given frequency
    pub fn new(timer: I, frequency: impl Into<Hertz>, rcc: &mut Rcc) -> Self {
        let frequency = frequency.into();

        I::enable(rcc);
        I::reset(rcc);

        let mut tim = Self {
            instance: timer,
            channel1: Pwm::new(),
            channel2: Pwm::new(),
            channel3: Pwm::new(),
            channel4: Pwm::new(),
        };
        tim.set_frequency(frequency, rcc);
        tim
    }

    /// Starts the PWM timer
    pub fn start(&mut self) {
        self.instance.cr1.write(|w| w.cen().set_bit());
    }

    /// Stops the PWM timer
    pub fn stop(&mut self) {
        self.instance.cr1.write(|w| w.cen().clear_bit());
    }

    /// Update frequency of the timer
    /// # Note
    /// In order to do this operation properly the function stop the timer and then starts it again.
    /// The duty cycle that was set before for given pin needs to adjusted according to the
    /// frequency
    pub fn set_frequency(&mut self, frequency: impl Into<Hertz>, rcc: &Rcc) {
        let frequency = frequency.into();
        self.stop();
        let (psc, arr) = get_clock_config(frequency.0, I::clock_frequency(rcc));
        self.instance.psc.write(|w| w.psc().bits(psc));
        self.instance.arr.write(|w| w.arr().bits(arr));
        self.start();
    }

    /// Returns the timer, so it can be used by any else
    pub fn free(self) -> I {
        self.instance
    }
}

fn get_clock_config(freq: u32, clk: u32) -> (u16, u16) {
    let ticks = clk / freq;
    let psc = u16((ticks - 1) / (1 << 16)).unwrap();
    let arr = u16(ticks / u32(psc + 1)).unwrap();
    (psc, arr)
}

pub trait Instance: Deref<Target = tim2::RegisterBlock> + Enable + Reset {
    fn ptr() -> *const tim2::RegisterBlock;
    fn clock_frequency(_: &Rcc) -> u32;
}

macro_rules! impl_instance {
    (
        $(
            $name:ty,
            $apbX_clk:ident;
        )*
    ) => {
        $(
            impl Instance for $name {
                fn ptr() -> *const tim2::RegisterBlock {
                    Self::ptr()
                }

                fn clock_frequency(rcc: &Rcc) -> u32 {
                    rcc.clocks.$apbX_clk().0
                }
            }
        )*
    }
}

impl_instance!(
    TIM2, apb1_clk;
    TIM3, apb1_clk;
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
                    tim.$ccmr_output().modify(|_, w| {
                        w.$ocxpe().set_bit();
                        w.$ocxm().bits(0b110)
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

pub struct Pwm<I, C, State> {
    channel: PhantomData<C>,
    timer: PhantomData<I>,
    _state: State,
}

impl<I, C> Pwm<I, C, Unassigned> {
    fn new() -> Self {
        Self {
            channel: PhantomData,
            timer: PhantomData,
            _state: Unassigned,
        }
    }

    pub fn assign<P>(self, pin: P) -> Pwm<I, C, Assigned<P>>
    where
        P: Pin<I, C>,
    {
        pin.setup();
        Pwm {
            channel: self.channel,
            timer: self.timer,
            _state: Assigned(pin),
        }
    }
}

impl<I, C, P> hal::PwmPin for Pwm<I, C, Assigned<P>>
where
    I: Instance,
    C: Channel,
{
    type Duty = u16;

    fn disable(&mut self) {
        interrupt::free(|_|
            // Safe, as the read-modify-write within the critical section
            C::disable(unsafe { &*I::ptr() }))
    }

    fn enable(&mut self) {
        interrupt::free(|_|
            // Safe, as the read-modify-write within the critical section
            C::enable(unsafe { &*I::ptr() }))
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

impl<I, C, P> Pwm<I, C, Assigned<P>>
where
    I: Instance,
    C: Channel,
{
    /// This allows to dynamically change the frequency of the underlying PWM timer.
    ///
    /// **WARNING:**
    /// This changes the frequency for all channels associated with the PWM timer.
    pub fn set_frequency(&mut self, frequency: Hertz, rcc: &Rcc) {
        let (psc, arr) = get_clock_config(frequency.0, I::clock_frequency(rcc));
        unsafe {
            (*I::ptr()).psc.write(|w| w.psc().bits(psc));
            (*I::ptr()).arr.write(|w| w.arr().bits(arr));
        }
    }
}
pub trait Pin<I, C> {
    fn setup(&self);
}

macro_rules! impl_pin {
    (
        $(
            $instance:ty: (
                $(
                    $name:ident,
                    $channel:ty,
                    $alternate_function:ident;
                )*
            )
        )*
    ) => {
        $(
            $(
                impl<State: PinMode> Pin<$instance, $channel> for $name<State> {
                    fn setup(&self) {
                        self.set_alt_mode(AltMode::$alternate_function);
                    }
                }
            )*
        )*
    }
}

impl_pin!(
    TIM2: (
        PA0, C1, AF2;
        PA1, C2, AF2;
        PA2, C3, AF2;
        PA3, C4, AF2;
    )
);

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
impl_pin!(
    TIM2: (
        PA5,  C1, AF5;
        PA15, C1, AF5;
        PB3,  C2, AF2;
        PB10, C3, AF2;
        PB11, C4, AF2;
    )
);

#[cfg(any(feature = "stm32l072", feature = "stm32l082", feature = "io-STM32L071",))]
impl_pin!(
    TIM3: (
        PA6, C1, AF2;
        PA7, C2, AF2;
        PB0, C3, AF2;
        PB1, C4, AF2;
        PB4, C1, AF2;
        PB5, C2, AF4;
    )
);

#[cfg(feature = "stm32l072")]
impl_pin!(
    TIM2: (
        PE9,  C1, AF0;
        PE10, C2, AF0;
        PE11, C3, AF0;
        PE12, C4, AF0;
    )
    TIM3: (
        PC6, C1, AF2;
        PC7, C2, AF2;
        PC8, C3, AF2;
        PC9, C4, AF2;
        PE3, C1, AF2;
        PE4, C2, AF2;
        PE5, C3, AF2;
        PE6, C4, AF2;
    )
);

/// Indicates that a PWM channel has not been assigned to a pin
pub struct Unassigned;

/// Indicates that a PWM channel has been assigned to the given pin
pub struct Assigned<P>(P);
