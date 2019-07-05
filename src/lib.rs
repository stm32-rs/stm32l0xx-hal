#![no_std]
#![allow(non_camel_case_types)]

#[cfg(not(any(feature = "stm32l0x1", feature = "stm32l0x2")))]
compile_error!("This crate requires one of the following features enabled: stm32l0x1, stm32l0x2");

use embedded_hal as hal;

#[cfg(feature = "stm32l0x1")]
pub use stm32l0::stm32l0x1 as pac;
#[cfg(feature = "stm32l0x2")]
pub use stm32l0::stm32l0x2 as pac;

pub use crate::pac as device;
pub use crate::pac as stm32;

mod bb;

#[cfg(feature = "stm32l0x1")]
pub mod adc;
#[cfg(any(feature = "stm32l062", feature = "stm32l082"))]
pub mod aes;
pub mod delay;
pub mod exti;
pub mod gpio;
pub mod i2c;
pub mod prelude;
pub mod pwm;
pub mod rcc;
pub mod serial;
pub mod spi;
pub mod time;
pub mod timer;
pub mod watchdog;
