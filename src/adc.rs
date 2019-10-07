//! # Analog to Digital converter
use crate::gpio::*;
use crate::hal::adc::{Channel, OneShot};
use crate::pac::ADC;
use crate::rcc::Rcc;

/// ADC Result Alignment
#[derive(PartialEq)]
pub enum Align {
    /// Right aligned results (least significant bits)
    ///
    /// Results in all precisions returning values from 0-(2^bits-1) in
    /// steps of 1.
    Right,
    /// Left aligned results (most significant bits)
    ///
    /// Results in all precisions returning a value in the range 0-65535.
    /// Depending on the precision the result will step by larger or smaller
    /// amounts.
    Left,
}

/// ADC Sampling Precision
#[derive(Copy, Clone, PartialEq)]
pub enum Precision {
    /// 12 bit precision
    B_12 = 0b00,
    /// 10 bit precision
    B_10 = 0b01,
    /// 8 bit precision
    B_8 = 0b10,
    /// 6 bit precision
    B_6 = 0b11,
}

/// ADC Sampling time
#[derive(Copy, Clone, PartialEq)]
pub enum SampleTime {
    /// 1.5 ADC clock cycles
    T_1_5 = 0b000,

    /// 3.5 ADC clock cycles
    T_3_5 = 0b001,

    /// 7.5 ADC clock cycles
    T_7_5 = 0b010,

    /// 12.5 ADC clock cycles
    T_12_5 = 0b011,

    /// 19.5 ADC clock cycles
    T_19_5 = 0b100,

    /// 39.5 ADC clock cycles
    T_39_5 = 0b101,

    /// 79.5 ADC clock cycles
    T_79_5 = 0b110,

    /// 160.5 ADC clock cycles
    T_160_5 = 0b111,
}

/// Analog to Digital converter interface
pub struct Adc<State> {
    rb: ADC,
    sample_time: SampleTime,
    align: Align,
    precision: Precision,
    _state: State,
}

impl Adc<Ready> {
    pub fn new(adc: ADC, rcc: &mut Rcc) -> Self {
        // Enable ADC clocks
        rcc.rb.apb2enr.modify(|_, w| w.adcen().set_bit());
        adc.cr.modify(|_, w| w.advregen().set_bit());

        Self {
            rb: adc,
            sample_time: SampleTime::T_1_5,
            align: Align::Right,
            precision: Precision::B_12,
            _state: Ready,
        }
    }

    /// Set the Adc sampling time
    pub fn set_sample_time(&mut self, t_samp: SampleTime) {
        self.sample_time = t_samp;
    }

    /// Set the Adc result alignment
    pub fn set_align(&mut self, align: Align) {
        self.align = align;
    }

    /// Set the Adc precision
    pub fn set_precision(&mut self, precision: Precision) {
        self.precision = precision;
    }

    fn power_up(&mut self) {
        self.rb.isr.modify(|_, w| w.adrdy().set_bit());
        self.rb.cr.modify(|_, w| w.aden().set_bit());
        while self.rb.isr.read().adrdy().bit_is_clear() {}
    }

    fn power_down(&mut self) {
        self.rb.cr.modify(|_, w| w.addis().set_bit());
        self.rb.isr.modify(|_, w| w.adrdy().set_bit());
        while self.rb.cr.read().aden().bit_is_set() {}
    }

    fn write_smpr(&mut self) {
        self.rb
            .smpr
            .modify(|_, w| w.smp().bits(self.sample_time as u8));
    }

    pub fn release(self) -> ADC {
        self.rb
    }
}

pub trait AdcExt {
    fn constrain(self, rcc: &mut Rcc) -> Adc<Ready>;
}

impl AdcExt for ADC {
    fn constrain(self, rcc: &mut Rcc) -> Adc<Ready> {
        Adc::new(self, rcc)
    }
}

impl<WORD, PIN> OneShot<Adc<Ready>, WORD, PIN> for Adc<Ready>
where
    WORD: From<u16>,
    PIN: Channel<Adc<Ready>, ID = u8>,
{
    type Error = ();

    fn read(&mut self, _pin: &mut PIN) -> nb::Result<WORD, Self::Error> {
        self.power_up();
        self.rb.cfgr1.modify(|_, w| {
            // Safe, as `self.precision` is of type `Precision`, which defines
            // only valid values.
            //
            // The `bits` method is not unsafe on STM32L0x1, so we need to
            // suppress the warning there.
            let w = w.res().bits(self.precision as u8);
            w.align().bit(self.align == Align::Left)
        });

        self.write_smpr();

        self.rb.chselr.write(|w|
            // Safe, as long as there are no `Channel` implementations that
            // define invalid values.
            unsafe { w.bits(0b1 << PIN::channel()) });

        self.rb.isr.modify(|_, w| w.eos().set_bit());
        self.rb.cr.modify(|_, w| w.adstart().set_bit());
        while self.rb.isr.read().eos().bit_is_clear() {}

        let res = self.rb.dr.read().bits() as u16;
        let val = if self.align == Align::Left && self.precision == Precision::B_6 {
            res << 8
        } else {
            res
        };

        self.power_down();
        Ok(val.into())
    }
}


/// Indicates that the ADC peripheral is ready
pub struct Ready;


macro_rules! int_adc {
    ($($Chan:ident: ($chan:expr, $en:ident)),+ $(,)*) => {
        $(
            pub struct $Chan;

            impl $Chan {
                pub fn new() -> Self {
                    Self {}
                }

                pub fn enable(&mut self, adc: &mut Adc<Ready>) {
                    adc.rb.ccr.modify(|_, w| w.$en().set_bit());
                }

                pub fn disable(&mut self, adc: &mut Adc<Ready>) {
                    adc.rb.ccr.modify(|_, w| w.$en().clear_bit());
                }
            }

            impl Channel<Adc<Ready>> for $Chan {
                type ID = u8;

                fn channel() -> u8 {
                    $chan
                }
            }
        )+
    };
}

macro_rules! adc_pins {
    ($($Chan:ty: ($pin:ty, $chan:expr)),+ $(,)*) => {
        $(
            impl Channel<Adc<Ready>> for $pin {
                type ID = u8;

                fn channel() -> u8 { $chan }
            }
        )+
    };
}

int_adc! {
    VTemp: (18, tsen),
    VRef: (17, vrefen),
}

adc_pins! {
    Channel0: (gpioa::PA0<Analog>, 0u8),
    Channel1: (gpioa::PA1<Analog>, 1u8),
    Channel2: (gpioa::PA2<Analog>, 2u8),
    Channel3: (gpioa::PA3<Analog>, 3u8),
    Channel4: (gpioa::PA4<Analog>, 4u8),
    Channel5: (gpioa::PA5<Analog>, 5u8),
    Channel6: (gpioa::PA6<Analog>, 6u8),
    Channel7: (gpioa::PA7<Analog>, 7u8),
    Channel8: (gpiob::PB0<Analog>, 8u8),
    Channel9: (gpiob::PB1<Analog>, 9u8),
}

#[cfg(all(feature = "stm32l052", any(feature = "lqfp64", feature = "tfbga64",),))]
adc_pins! {
    Channel10: (gpioc::PC0<Analog>, 10u8),
    Channel11: (gpioc::PC1<Analog>, 11u8),
    Channel12: (gpioc::PC2<Analog>, 12u8),
}

#[cfg(all(
    feature = "stm32l072",
    any(
        feature = "lqfp64",
        feature = "lqfp100",
        feature = "tfbga64",
        feature = "ufbga64",
        feature = "ufbga100",
        feature = "wlcsp49",
    ),
))]
adc_pins! {
    Channel10: (gpioc::PC0<Analog>, 10u8),
    Channel11: (gpioc::PC1<Analog>, 11u8),
    Channel12: (gpioc::PC2<Analog>, 12u8),
}

#[cfg(all(feature = "stm32l082", feature = "wlcsp49"))]
adc_pins! {
    Channel10: (gpioc::PC0<Analog>, 10u8),
    Channel11: (gpioc::PC1<Analog>, 11u8),
    Channel12: (gpioc::PC2<Analog>, 12u8),
}

#[cfg(all(feature = "stm32l052", feature = "lqfp64"))]
adc_pins! {
    Channel13: (gpioc::PC3<Analog>, 13u8),
}

#[cfg(all(
    feature = "stm32l072",
    any(feature = "lqfp64", feature = "lqfp100", feature = "ufbga100",),
))]
adc_pins! {
    Channel13: (gpioc::PC3<Analog>, 13u8),
}

#[cfg(all(feature = "stm32l052", any(feature = "lqfp64", feature = "tfbga64",),))]
adc_pins! {
    Channel14: (gpioc::PC4<Analog>, 14u8),
    Channel15: (gpioc::PC5<Analog>, 15u8),
}

#[cfg(all(
    feature = "stm32l072",
    any(
        feature = "lqfp64",
        feature = "lqfp100",
        feature = "tfbga64",
        feature = "ufbga64",
        feature = "ufbga100",
    ),
))]
adc_pins! {
    Channel14: (gpioc::PC4<Analog>, 14u8),
    Channel15: (gpioc::PC5<Analog>, 15u8),
}
