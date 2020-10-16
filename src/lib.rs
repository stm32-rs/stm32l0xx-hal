#![cfg_attr(not(test), no_std)]
#![allow(non_camel_case_types)]

#[cfg(not(any(feature = "stm32l0x1", feature = "stm32l0x2", feature = "stm32l0x3")))]
compile_error!(
    "This crate requires one of the following features enabled: stm32l0x1, stm32l0x2, stm32l0x3"
);

use embedded_hal as hal;

#[cfg(feature = "stm32l0x1")]
pub use stm32l0::stm32l0x1 as pac;
#[cfg(feature = "stm32l0x2")]
pub use stm32l0::stm32l0x2 as pac;
#[cfg(feature = "stm32l0x3")]
pub use stm32l0::stm32l0x3 as pac;

pub mod adc;
pub mod aes;
pub mod calibration;
pub mod delay;
pub mod dma;
pub mod exti;
#[cfg(feature = "stm32l0x2")]
pub mod flash;
pub mod gpio;
#[cfg(any(
    feature = "io-STM32L021",
    feature = "io-STM32L031",
    feature = "io-STM32L051",
    feature = "io-STM32L071",
))]
pub mod i2c;
pub mod lptim;
pub mod prelude;
pub mod pwm;
pub mod pwr;
pub mod rcc;
#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
pub mod rng;
pub mod rtc;
#[cfg(any(
    feature = "io-STM32L021",
    feature = "io-STM32L031",
    feature = "io-STM32L051",
    feature = "io-STM32L071",
))]
pub mod serial;
pub mod spi;
pub mod syscfg;
pub mod time;
pub mod timer;
#[cfg(all(
    feature = "stm32-usbd",
    any(feature = "stm32l0x2", feature = "stm32l0x3")
))]
pub mod usb;
pub mod watchdog;
