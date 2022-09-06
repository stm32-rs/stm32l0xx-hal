//! External interrupt controller (EXTI).
//!
//! For convenience, this module reexports the EXTI peripheral from the PAC.

use crate::pac::EXTI;
use crate::pwr::PowerMode;
use crate::syscfg::SYSCFG;
use crate::{gpio, pac};
use cortex_m::{interrupt, peripheral::NVIC};

/// Edges that can trigger a configurable interrupt line.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TriggerEdge {
    /// Trigger on rising edges only.
    Rising,
    /// Trigger on falling edges only.
    Falling,
    /// Trigger on both rising and falling edges.
    Both,
}

/// Higher-lever wrapper around the `EXTI` peripheral.
pub struct Exti {
    raw: EXTI,
}

impl Exti {
    /// Creates a new `Exti` wrapper from the raw `EXTI` peripheral.
    pub fn new(raw: EXTI) -> Self {
        Self { raw }
    }

    /// Destroys this `Exti` instance, returning the raw `EXTI` peripheral.
    pub fn release(self) -> EXTI {
        self.raw
    }

    /// Starts listening on a GPIO interrupt line.
    ///
    /// GPIO interrupt lines are "configurable" lines, meaning that the edges
    /// that should trigger the interrupt can be configured. However, they
    /// require more setup than ordinary "configurable" lines, which requires
    /// access to the `SYSCFG` peripheral.
    // `port` and `line` are almost always constants, so make sure they can get
    // constant-propagated by inlining the method. Saves ~600 Bytes in the
    // `lptim.rs` example.
    #[inline]
    pub fn listen_gpio(
        &mut self,
        syscfg: &mut SYSCFG,
        port: gpio::Port,
        line: GpioLine,
        edge: TriggerEdge,
    ) {
        let line = line.raw_line();

        // translate port into bit values for EXTIn registers
        let port_bm = match port {
            gpio::Port::PA => 0,
            gpio::Port::PB => 1,
            gpio::Port::PC => 2,
            gpio::Port::PD => 3,
            gpio::Port::PE => 4,
            gpio::Port::PH => {
                assert!((line < 2) | (line == 9) | (line == 10));
                5
            }
        };

        unsafe {
            match line {
                0 | 1 | 2 | 3 => {
                    syscfg.syscfg.exticr1.modify(|_, w| match line {
                        0 => w.exti0().bits(port_bm),
                        1 => w.exti1().bits(port_bm),
                        2 => w.exti2().bits(port_bm),
                        3 => w.exti3().bits(port_bm),
                        _ => w,
                    });
                }
                4 | 5 | 6 | 7 => {
                    // no need to assert that PH is not port,
                    // since line is assert on port above
                    syscfg.syscfg.exticr2.modify(|_, w| match line {
                        4 => w.exti4().bits(port_bm),
                        5 => w.exti5().bits(port_bm),
                        6 => w.exti6().bits(port_bm),
                        7 => w.exti7().bits(port_bm),
                        _ => w,
                    });
                }
                8 | 9 | 10 | 11 => {
                    syscfg.syscfg.exticr3.modify(|_, w| match line {
                        8 => w.exti8().bits(port_bm),
                        9 => w.exti9().bits(port_bm),
                        10 => w.exti10().bits(port_bm),
                        11 => w.exti11().bits(port_bm),
                        _ => w,
                    });
                }
                12 | 13 | 14 | 15 => {
                    syscfg.syscfg.exticr4.modify(|_, w| match line {
                        12 => w.exti12().bits(port_bm),
                        13 => w.exti13().bits(port_bm),
                        14 => w.exti14().bits(port_bm),
                        15 => w.exti15().bits(port_bm),
                        _ => w,
                    });
                }
                _ => (),
            };
        }

        let bm: u32 = 1 << line;

        unsafe {
            match edge {
                TriggerEdge::Rising => self.raw.rtsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Falling => self.raw.ftsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Both => {
                    self.raw.rtsr.modify(|r, w| w.bits(r.bits() | bm));
                    self.raw.ftsr.modify(|r, w| w.bits(r.bits() | bm));
                }
            }

            self.raw.imr.modify(|r, w| w.bits(r.bits() | bm));
        }
    }

    /// Starts listening on a configurable interrupt line.
    ///
    /// The edges that should trigger the interrupt can be configured with
    /// `edge`.
    #[inline]
    pub fn listen_configurable(&mut self, line: ConfigurableLine, edge: TriggerEdge) {
        let bm: u32 = 1 << line.raw_line();

        unsafe {
            match edge {
                TriggerEdge::Rising => self.raw.rtsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Falling => self.raw.ftsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Both => {
                    self.raw.rtsr.modify(|r, w| w.bits(r.bits() | bm));
                    self.raw.ftsr.modify(|r, w| w.bits(r.bits() | bm));
                }
            }

            self.raw.imr.modify(|r, w| w.bits(r.bits() | bm));
        }
    }

    /// Starts listening on a "direct" interrupt line.
    #[inline]
    pub fn listen_direct(&mut self, line: DirectLine) {
        let bm: u32 = 1 << line.raw_line();

        unsafe {
            self.raw.imr.modify(|r, w| w.bits(r.bits() | bm));
        }
    }

    /// Disables the interrupt on `line`.
    pub fn unlisten<L: ExtiLine>(&mut self, line: L) {
        let bm = 1 << line.raw_line();

        // Safety: We clear the correct bit and have unique ownership of the EXTI registers here.
        unsafe {
            self.raw.imr.modify(|r, w| w.bits(r.bits() & !bm));
            self.raw.rtsr.modify(|r, w| w.bits(r.bits() & !bm));
            self.raw.ftsr.modify(|r, w| w.bits(r.bits() & !bm));
        }
    }

    /// Marks `line` as "pending".
    ///
    /// This will cause an interrupt if the EXTI was previously configured to
    /// listen on `line`.
    ///
    /// If `line` is already pending, this does nothing.
    pub fn pend<L: ExtiLine>(line: L) {
        let line = line.raw_line();

        // Safety:
        // - We've ensured that the only 1-bit written is a valid line.
        // - This mirrors the `NVIC::pend` API and implementation, which is
        //   presumed safe.
        // - This is a "set by writing 1" register (ie. writing 0 does nothing),
        //   and this is a single write operation that cannot be interrupted.
        unsafe {
            (*EXTI::ptr()).swier.write(|w| w.bits(1 << line));
        }
    }

    /// Marks `line` as "not pending".
    ///
    /// This should be called from an interrupt handler to ensure that the
    /// interrupt doesn't continuously fire.
    pub fn unpend<L: ExtiLine>(line: L) {
        let line = line.raw_line();

        // Safety:
        // - We've ensured that the only 1-bit written is a valid line.
        // - This mirrors the `NVIC::pend` API and implementation, which is
        //   presumed safe.
        // - This is a "clear by writing 1" register, and this is a single write
        //   operation that cannot be interrupted.
        unsafe {
            (*EXTI::ptr()).pr.write(|w| w.bits(1 << line));
        }
    }

    /// Returns whether `line` is currently marked as pending.
    pub fn is_pending<L: ExtiLine>(line: L) -> bool {
        let bm: u32 = 1 << line.raw_line();

        // Safety: This is a read without side effects that cannot be
        // interrupted.
        let pr = unsafe { (*EXTI::ptr()).pr.read().bits() };

        pr & bm != 0
    }

    /// Enters a low-power mode until an interrupt occurs.
    ///
    /// Please note that this method will return after _any_ interrupt that can
    /// wake up the microcontroller from the given power mode.
    pub fn wait_for_irq<L, M>(&mut self, line: L, mut power_mode: M)
    where
        L: ExtiLine,
        M: PowerMode,
    {
        let interrupt = line.interrupt();

        // This construct allows us to wait for the interrupt without having to
        // define an interrupt handler.
        interrupt::free(|_| {
            // Safety: Interrupts are globally disabled, and we re-mask and unpend the interrupt
            // before reenabling interrupts and returning.
            unsafe {
                NVIC::unmask(interrupt);
            }

            power_mode.enter();

            Self::unpend(line);
            NVIC::unpend(interrupt);
            NVIC::mask(interrupt);
        });
    }
}

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
///
/// You can create a `GpioLine` by using the `ExtiLine::from_raw_line` method.
/// Lines `0..=15` are valid GPIO lines.
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
