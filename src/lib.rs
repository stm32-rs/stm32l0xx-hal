#![no_std]

#[cfg(not(any(feature = "stm32l0x1")))]
compile_error!("This crate requires one of the following features enabled: stm32l0x1");

use embedded_hal as hal;

#[cfg(feature = "stm32l0x1")]
pub use stm32l0::stm32l0x1 as pac;

pub use crate::pac as device;
pub use crate::pac as stm32;

mod bb;

pub mod adc;
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
