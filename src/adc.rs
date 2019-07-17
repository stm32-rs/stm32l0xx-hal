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
pub struct Adc {
    rb: ADC,
    sample_time: SampleTime,
    align: Align,
    precision: Precision,
}

impl Adc {
    pub fn new(adc: ADC, rcc: &mut Rcc) -> Self {
        // Enable ADC clocks
        rcc.rb.apb2enr.modify(|_, w| w.adcen().set_bit());
        adc.cr.modify(|_, w| w.advregen().set_bit());

        Self {
            rb: adc,
            sample_time: SampleTime::T_1_5,
            align: Align::Right,
            precision: Precision::B_12,
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

    #[cfg(feature = "stm32l0x1")]
    fn write_smpr(&mut self) {
        self.rb
            .smpr
            .modify(|_, w| w.smp().bits(self.sample_time as u8));
    }

    #[cfg(feature = "stm32l0x2")]
    fn write_smpr(&mut self) {
        self.rb
            .smpr
            // Safe, because `self.sample_time` is of type `SampleTime`, which
            // defines only valid values.
            .modify(|_, w| unsafe { w.smpr().bits(self.sample_time as u8) });
    }

    pub fn release(self) -> ADC {
        self.rb
    }
}

pub trait AdcExt {
    fn constrain(self, rcc: &mut Rcc) -> Adc;
}

impl AdcExt for ADC {
    fn constrain(self, rcc: &mut Rcc) -> Adc {
        Adc::new(self, rcc)
    }
}

impl<WORD, PIN> OneShot<Adc, WORD, PIN> for Adc
where
    WORD: From<u16>,
    PIN: Channel<Adc, ID = u8>,
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
            #[cfg_attr(feature = "stm32l0x1", allow(unused_unsafe))]
            let w = unsafe { w.res().bits(self.precision as u8) };
            w
                .align().bit(self.align == Align::Left)
        });

        self.write_smpr();

        match PIN::channel() {
            0 => self.rb.chselr.modify(|_, w| w.chsel0().set_bit()),
            1 => self.rb.chselr.modify(|_, w| w.chsel1().set_bit()),
            2 => self.rb.chselr.modify(|_, w| w.chsel2().set_bit()),
            3 => self.rb.chselr.modify(|_, w| w.chsel3().set_bit()),
            4 => self.rb.chselr.modify(|_, w| w.chsel4().set_bit()),
            5 => self.rb.chselr.modify(|_, w| w.chsel5().set_bit()),
            6 => self.rb.chselr.modify(|_, w| w.chsel6().set_bit()),
            7 => self.rb.chselr.modify(|_, w| w.chsel7().set_bit()),
            8 => self.rb.chselr.modify(|_, w| w.chsel8().set_bit()),
            9 => self.rb.chselr.modify(|_, w| w.chsel9().set_bit()),
            17 => self.rb.chselr.modify(|_, w| w.chsel17().set_bit()),
            18 => self.rb.chselr.modify(|_, w| w.chsel18().set_bit()),
            _ => unreachable!(),
        }

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

macro_rules! int_adc {
    ($($Chan:ident: ($chan:expr, $en:ident)),+ $(,)*) => {
        $(
            pub struct $Chan;

            impl $Chan {
                pub fn new() -> Self {
                    Self {}
                }

                pub fn enable(&mut self, adc: &mut Adc) {
                    adc.rb.ccr.modify(|_, w| w.$en().set_bit());
                }

                pub fn disable(&mut self, adc: &mut Adc) {
                    adc.rb.ccr.modify(|_, w| w.$en().clear_bit());
                }
            }

            impl Channel<Adc> for $Chan {
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
            impl Channel<Adc> for $pin {
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
