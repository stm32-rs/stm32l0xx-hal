#![no_std]
#![allow(non_camel_case_types)]

extern crate bare_metal;
extern crate cast;
extern crate cortex_m;
extern crate void;

pub extern crate embedded_hal as hal;
pub extern crate nb;
pub use stm32l0;

pub use nb::block;

pub use stm32l0::stm32l0x1 as stm32;

pub mod gpio;
pub mod prelude;
pub mod rcc;
pub mod time;