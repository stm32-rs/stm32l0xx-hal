//!  Measure the internal mcu temperature sensor and an analog external analog TMP36 temperature sensor.

//  This example compiles but has not been tested on actual hardware, and may not work.
//  If you have hardware and test it then please report results to issue
//      https://github.com/stm32-rs/stm32l0xx-hal/issues/161.
// (The issue may be closed but you should still be able to comment.)

//  This example is extracted from https://github.com/pdgilbert/eg_stm_hal/examples/temperature.rs,
//  which also has setup functions for several other device hals.

// It may be necessary to compile --release for the bin to fit flash.

#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_semihosting as _;

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;

// Traits to be supported by methods on sensors. Self refers to a Sensor which is an optional pin.
// It would be possible to have just one trait, eg ReadSensor, that included both read_tempc and read_mv
//  but then both need to be implemented, which is not done below.

// possibly trait cfg's could be eliminated by using <T> or <T: Adcs> or Type: item =  x; ??

pub trait ReadTempC {
    // for reading channel temperature in degrees C on channel (self.ch)
    fn read_tempc(&mut self, adcs: &mut Adc<Ready>) -> i32;
}

pub trait ReadMV {
    fn read_mv(&mut self, adcs: &mut Adc<Ready>) -> u32;
}

pub struct Sensor<U> {
    // ch is None for internal temp
    ch: U,
}

// setup() does all  hal/MCU specific setup and returns generic objects for use in the main code.

use stm32l0xx_hal::{
    adc::{Adc, Ready, VTemp},
    gpio::{gpiob::PB1, Analog},
    pac::Peripherals,
    prelude::*,
    rcc, // for ::Config
};

fn setup() -> (impl ReadTempC, impl ReadTempC + ReadMV, Adc<Ready>) {
    // On stm32L0X2 a temperature sensor is internally connected to the single adc.
    // No channel is specified for the mcutemp because it uses an internal channel ADC_IN18.

    let p = Peripherals::take().unwrap();
    let mut rcc = p.RCC.freeze(rcc::Config::hsi16());
    let adc = p.ADC.constrain(&mut rcc);

    let gpiob = p.GPIOB.split(&mut rcc);

    //The MCU temperature sensor is internally connected to the ADC12_IN16 input channel
    // so no channel needs to be specified here.

    let mcutemp: Sensor<Option<PB1<Analog>>> = Sensor { ch: None }; // no channel

    let tmp36: Sensor<Option<PB1<Analog>>> = Sensor {
        ch: Some(gpiob.pb1.into_analog()), //channel pb1
    };

    impl ReadTempC for Sensor<Option<PB1<Analog>>> {
        fn read_tempc(&mut self, a: &mut Adc<Ready>) -> i32 {
            match &mut self.ch {
                Some(ch) => {
                    let v: f32 = a.read(ch).unwrap();
                    (v / 12.412122) as i32 - 50 as i32
                }

                None => {
                    let v: f32 = a.read(&mut VTemp).unwrap();
                    (v / 12.412122) as i32 - 50 as i32 //CHECK THIS
                }
            }
        }
    }

    impl ReadMV for Sensor<Option<PB1<Analog>>> {
        // TMP36 on PB1 using ADC2
        fn read_mv(&mut self, a: &mut Adc<Ready>) -> u32 {
            match &mut self.ch {
                Some(ch) => a.read(ch).unwrap(),
                None => panic!(),
            }
        }
    }

    (mcutemp, tmp36, adc)
}

#[entry]
fn main() -> ! {
    let (mut mcutemp, mut tmp36, mut adc) = setup();

    //  TMP35 has linear output with scale calculation as follows.
    //  Vin = 3.3v * ADCvalue / 4096     (12 bit adc has  2**12 = 4096 steps)
    //  TMP35 scale is 100 deg C per 1.0v (slope 10mV/deg C) and goes through
    //     <50C, 1.0v>,  so 0.0v is  -50C.
    //  see https://www.analog.com/media/en/technical-documentation/data-sheets/TMP35_36_37.pdf
    //  so temp = (100 * 3.3 * ADCvalue / 4096 )  - 50 = 0.0805664 * ADCvalue - 50

    loop {
        let mcu_value = mcutemp.read_tempc(&mut adc);
        hprintln!("inaccurate MCU temp: {}", mcu_value).unwrap();

        let tmp36_mv: u32 = tmp36.read_mv(&mut adc);
        let tmp36_temp: i32 = tmp36.read_tempc(&mut adc);
        hprintln!("external sensor: {} mV,   {} C.", tmp36_mv, tmp36_temp).unwrap();
    }
}
