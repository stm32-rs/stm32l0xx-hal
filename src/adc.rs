//! # Analog to Digital converter
use crate::gpio::*;
use crate::hal::adc::{Channel, OneShot};
use crate::rcc::Rcc;
use crate::stm32::ADC;

/// Analog to Digital converter interface
pub struct Adc {
    rb: ADC,
    sample_time: SampleTime,
    align: Align,
    precision: Precision,
}

/// Internal temperature sensor (ADC Channel 18)
pub struct VTemp;

/// Internal voltage reference (ADC Channel 17)
pub struct VRef;

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
#[allow(non_camel_case_types)]
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
#[allow(non_camel_case_types)]
pub enum SampleTime {
    T_4 = 0b000,
    T_9 = 0b001,
    T_16 = 0b010,
    T_24 = 0b011,
    T_48 = 0b100,
    T_96 = 0b101,
    T_192 = 0b110,
    T_384 = 0b111,
}

impl Adc {
    pub fn new(adc: ADC, rcc: &mut Rcc) -> Self {
        // Enable HSI
        rcc.rb.cr.write(|w| w.hsi16on().set_bit());
        while rcc.rb.cr.read().hsi16rdyf().bit_is_clear() {}

        // Enable ADC clocks
        rcc.rb.apb2enr.modify(|_, w| w.adcen().set_bit());

        Self {
            rb: adc,
            sample_time: SampleTime::T_4,
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
        if self.rb.cr.read().aden().bit_is_set() {
            self.power_down();
        }
        self.rb.cr.modify(|_, w| w.aden().set_bit());
        while self.rb.cr.read().aden().bit_is_clear() {}
    }

    fn power_down(&mut self) {
        self.rb.cr.modify(|_, w| w.addis().set_bit());
    }
}

pub trait AdcChannel {
    fn setup(&mut self, adc: &mut Adc);
}

// TODO: NEEDS TESTING
macro_rules! adc_pins {
    ($($Chan:ty: ($pin:ty, $bank_b:tt, $chan:expr, $smprx:ident)),+ $(,)*) => {
        $(
            impl Channel<Adc> for $pin {
                type ID = u8;

                fn channel() -> u8 { $chan }
            }

            impl AdcChannel for $pin {
                fn setup(&mut self, adc: &mut Adc) {
                    adc.rb.$smprx.modify(|r, w| unsafe {
                        const OFFSET: u8 = 3 * $chan % 10;
                        let mut bits = r.smp().bits() as u32;
                        bits &= !(0xfff << OFFSET);
                        bits |= (adc.sample_time as u32) << OFFSET;
                        w.bits(bits)
                    });
                }
            }
        )+
    };
}

adc_pins! {
    Channel0: (gpioa::PA0<Analog>, false, 0_u8, smpr),
    Channel1: (gpioa::PA1<Analog>, false, 1_u8, smpr),
    Channel2: (gpioa::PA2<Analog>, false, 2_u8, smpr),
    Channel3: (gpioa::PA3<Analog>, false, 3_u8, smpr),
    Channel4: (gpioa::PA4<Analog>, false, 4_u8, smpr),
    Channel5: (gpioa::PA5<Analog>, false, 5_u8, smpr),
    Channel6: (gpioa::PA6<Analog>, false, 6_u8, smpr),
    Channel7: (gpioa::PA7<Analog>, false, 7_u8, smpr),
    Channel8: (gpiob::PB0<Analog>, false, 8_u8, smpr),
    Channel9: (gpiob::PB1<Analog>, false, 9_u8, smpr),
}

impl VTemp {
    /// Init a new VTemp
    pub fn new() -> Self {
        VTemp {}
    }

    /// Enable the internal temperature sense
    pub fn enable(&mut self, adc: &mut Adc) {
        adc.rb.ccr.modify(|_, w| w.tsen().set_bit());
    }

    /// Disable the internal temperature sense.
    pub fn disable(&mut self, adc: &mut Adc) {
        adc.rb.ccr.modify(|_, w| w.tsen().clear_bit());
    }
}

impl VRef {
    /// Init a new VRef
    pub fn new() -> Self {
        VRef {}
    }

    /// Enable the internal voltage reference, remember to disable when not in use.
    pub fn enable(&mut self, adc: &mut Adc) {
        adc.rb.ccr.modify(|_, w| w.vrefen().set_bit());
    }

    /// Disable the internal reference voltage.
    pub fn disable(&mut self, adc: &mut Adc) {
        adc.rb.ccr.modify(|_, w| w.vrefen().clear_bit());
    }
}

impl Channel<Adc> for VTemp {
    type ID = u8;

    fn channel() -> u8 {
        18
    }
}

impl Channel<Adc> for VRef {
    type ID = u8;

    fn channel() -> u8 {
        17
    }
}

impl<WORD, PIN> OneShot<Adc, WORD, PIN> for Adc
where
    WORD: From<u16>,
    PIN: AdcChannel + Channel<Adc, ID = u8>,
{
    type Error = ();

    fn read(&mut self, pin: &mut PIN) -> nb::Result<WORD, Self::Error> {
        self.power_up();
        pin.setup(self);

        self.rb.cr.modify(|_, w| w.adstart().set_bit());
        while self.rb.isr.read().eoc().bit_is_clear() {}

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

pub trait AdcExt {
    fn adc(self, rcc: &mut Rcc) -> Adc;
}

impl AdcExt for ADC {
    fn adc(self, rcc: &mut Rcc) -> Adc {
        Adc::new(self, rcc)
    }
}
