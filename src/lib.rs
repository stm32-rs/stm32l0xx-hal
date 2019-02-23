#![no_std]
#![allow(non_camel_case_types)]

#[cfg(not(any(
    feature = "stm32l0x1"
)))]
compile_error!("This crate requires one of the following features enabled: stm32l0x1");


extern crate bare_metal;
extern crate cast;
extern crate cortex_m;
extern crate void;

extern crate embedded_hal as hal;
pub extern crate nb;
pub use stm32l0;

pub use nb::block;

#[cfg(feature = "stm32l0x1")]
pub use stm32l0::stm32l0x1 as stm32;

#[cfg(feature = "rt")]
pub use crate::stm32::interrupt;

pub mod adc;
pub mod delay;
pub mod gpio;
pub mod prelude;
pub mod pwm;
pub mod rcc;
pub mod time;
pub mod timer;
pub mod watchdog;