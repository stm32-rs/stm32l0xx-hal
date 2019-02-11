#![no_std]
#![allow(non_camel_case_types)]

#[cfg(not(any(
    feature = "stm32l011"
)))]
compile_error!("This crate requires one of the following features enabled: stm32l011");


extern crate bare_metal;
extern crate cast;
extern crate cortex_m;
extern crate void;

pub extern crate embedded_hal as hal;
pub extern crate nb;
pub use stm32l0;

pub use nb::block;

#[cfg(feature = "stm32l011")]
pub use stm32l0::stm32l0x1 as stm32;

#[cfg(feature = "rt")]
pub use crate::stm32::interrupt;

pub mod gpio;
pub mod prelude;
pub mod rcc;
pub mod time;