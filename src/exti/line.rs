//! Types for representing the EXTI input lines.

use crate::pac;

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::GpioLine {}
    impl Sealed for super::ConfigurableLine {}
    impl Sealed for super::DirectLine {}
}

/// Trait implemented by all types representing EXTI interrupt lines.
pub trait ExtiLine: Sized + sealed::Sealed {
    /// Returns the line object corresponding to a raw EXTI line number.
    ///
    /// If `raw` is not a valid line for type `Self`, `None` is returned.
    fn from_raw_line(raw: u8) -> Option<Self>;

    /// Returns that raw EXTI line number corresponding to `self`.
    fn raw_line(&self) -> u8;

    /// Returns the NVIC interrupt corresponding to `self`.
    fn interrupt(&self) -> pac::Interrupt;
}

/// An EXTI interrupt line sourced by a GPIO.
///
/// All `GpioLine`s are *configurable*: They can be configured to listen for
/// rising or falling edges.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct GpioLine(u8);

impl ExtiLine for GpioLine {
    fn from_raw_line(line: u8) -> Option<Self> {
        match line {
            0..=15 => Some(GpioLine(line)),
            _ => None,
        }
    }

    fn raw_line(&self) -> u8 {
        self.0
    }

    fn interrupt(&self) -> pac::Interrupt {
        use pac::Interrupt::*;
        match self.0 {
            0..=1 => EXTI0_1,
            2..=3 => EXTI2_3,
            _ => EXTI4_15,
        }
    }
}

/// A configurable EXTI line that is not a GPIO-sourced line.
///
/// These lines can be configured to listen for rising edges, falling edges, or
/// both.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ConfigurableLine {
    Pvd = 16,
    RtcAlarm = 17,
    RtcTamper_CssLse = 19,
    RtcWakeup = 20,
    Comp1 = 21,
    Comp2 = 22,
}

impl ExtiLine for ConfigurableLine {
    fn from_raw_line(line: u8) -> Option<Self> {
        use ConfigurableLine::*;

        Some(match line {
            16 => Pvd,
            17 => RtcAlarm,
            // 18 = USB (or reserved)
            19 => RtcTamper_CssLse,
            20 => RtcWakeup,
            21 => Comp1,
            22 => Comp2,
            _ => return None,
        })
    }

    fn raw_line(&self) -> u8 {
        *self as u8
    }

    fn interrupt(&self) -> pac::Interrupt {
        use pac::Interrupt;
        use ConfigurableLine::*;

        match self {
            Pvd => Interrupt::PVD,
            RtcAlarm | RtcTamper_CssLse | RtcWakeup => Interrupt::RTC,
            Comp1 | Comp2 => Interrupt::ADC_COMP,
        }
    }
}

/// A non-configurable interrupt line.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DirectLine {
    #[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
    Usb = 18,
    I2C1 = 23,
    I2C3 = 24,
    Usart1 = 25,
    Usart2 = 26,
    // 27 = reserved
    Lpuart1 = 28,
    Lptim1 = 29,
}

impl ExtiLine for DirectLine {
    fn from_raw_line(line: u8) -> Option<Self> {
        use DirectLine::*;

        Some(match line {
            #[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
            18 => Usb,
            23 => I2C1,
            24 => I2C3,
            25 => Usart1,
            26 => Usart2,
            // 27 = reserved
            28 => Lpuart1,
            29 => Lptim1,
            _ => return None,
        })
    }

    fn raw_line(&self) -> u8 {
        *self as u8
    }

    fn interrupt(&self) -> pac::Interrupt {
        use pac::Interrupt;
        use DirectLine::*;

        match self {
            #[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
            Usb => Interrupt::USB,
            I2C1 => Interrupt::I2C1,
            I2C3 => Interrupt::I2C3,
            Usart1 => Interrupt::USART1,
            Usart2 => Interrupt::USART2,
            Lpuart1 => Interrupt::AES_RNG_LPUART1,
            Lptim1 => Interrupt::LPTIM1,
        }
    }
}
