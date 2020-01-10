//! External interrupt controller (EXTI).
//!
//! For convenience, this module reexports the EXTI peripheral from the PAC.

pub mod line;

use self::line::{ExtiLine, GpioLine, ConfigurableLine, DirectLine};

use crate::gpio;
use crate::pwr::PowerMode;
use crate::syscfg::SYSCFG;

use cortex_m::{interrupt, peripheral::NVIC};

pub use crate::pac::EXTI;

/// Edges that can trigger a configurable interrupt line.
pub enum TriggerEdge {
    /// Trigger on rising edges only.
    Rising,
    /// Trigger on falling edges only.
    Falling,
    /// Trigger on both rising and falling edges.
    Both,
}

/// Extension trait providing a high-level API for the EXTI controller.
pub trait ExtiExt {
    fn listen_gpio(&mut self, syscfg: &mut SYSCFG, port: gpio::Port, line: GpioLine, edge: TriggerEdge);
    fn listen_configurable(&mut self, line: ConfigurableLine, edge: TriggerEdge);
    fn listen_direct(&mut self, line: DirectLine);

    fn unlisten<L: ExtiLine>(&mut self, line: L);

    fn pend<L: ExtiLine>(line: L);
    fn unpend<L: ExtiLine>(line: L);
    fn is_pending<L: ExtiLine>(line: L) -> bool;

    fn wait_for_irq<L, M>(&mut self, line: L, power_mode: M)
        where L: ExtiLine, M: PowerMode;
}

impl ExtiExt for EXTI {
    /// Starts listening to a GPIO interrupt line.
    ///
    /// GPIO interrupt lines are "configurable" lines, meaning that the edges
    /// that should trigger the interrupt can be configured. However, they
    /// require more setup than ordinary "configurable" lines, which requires
    /// access to the `SYSCFG` peripheral.
    // `port` and `line` are almost always constants, so make sure they can get constant-propagated
    // by inlining the method. Saves ~600 Bytes in the `lptim.rs` example.
    #[inline]
    fn listen_gpio(&mut self, syscfg: &mut SYSCFG, port: gpio::Port, line: GpioLine, edge: TriggerEdge) {
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
                TriggerEdge::Rising => self.rtsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Falling => self.ftsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Both => {
                    self.rtsr.modify(|r, w| w.bits(r.bits() | bm));
                    self.ftsr.modify(|r, w| w.bits(r.bits() | bm));
                }
            }

            self.imr.modify(|r, w| w.bits(r.bits() | bm));
        }
    }

    /// Starts listening to a configurable interrupt line.
    ///
    /// The edges that should trigger the interrupt can be configured with
    /// `edge`.
    #[inline]
    fn listen_configurable(&mut self, line: ConfigurableLine, edge: TriggerEdge) {
        let bm: u32 = 1 << line.raw_line();

        unsafe {
            match edge {
                TriggerEdge::Rising => self.rtsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Falling => self.ftsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Both => {
                    self.rtsr.modify(|r, w| w.bits(r.bits() | bm));
                    self.ftsr.modify(|r, w| w.bits(r.bits() | bm));
                }
            }

            self.imr.modify(|r, w| w.bits(r.bits() | bm));
        }
    }

    /// Starts listening to a "direct" interrupt line.
    #[inline]
    fn listen_direct(&mut self, line: DirectLine) {
        let bm: u32 = 1 << line.raw_line();

        unsafe {
            self.imr.modify(|r, w| w.bits(r.bits() | bm));
        }
    }

    /// Disables the interrupt on `line`.
    fn unlisten<L: ExtiLine>(&mut self, line: L) {
        let bm = 1 << line.raw_line();

        // Safety: We clear the correct bit and have unique ownership of the EXTI registers here.
        unsafe {
            self.imr.modify(|r, w| w.bits(r.bits() & !bm));
            self.rtsr.modify(|r, w| w.bits(r.bits() & !bm));
            self.ftsr.modify(|r, w| w.bits(r.bits() & !bm));
        }
    }

    /// Marks `line` as "pending".
    ///
    /// This will cause an interrupt if the EXTI was previously configured to
    /// listen on `line`.
    ///
    /// If `line` is already pending, this does nothing.
    fn pend<L: ExtiLine>(line: L) {
        let line = line.raw_line();

        // Safety:
        // - We've ensured that the only 1-bit written is a valid line.
        // - This mirrors the `NVIC::pend` API and implementation, which is
        //   presumed safe.
        // - This is a "set by writing 1" register (ie. writing 0 does nothing),
        //   and this is a single write operation that cannot be interrupted.
        unsafe {
            (*Self::ptr()).swier.write(|w| w.bits(1 << line));
        }
    }

    /// Marks `line` as "not pending".
    fn unpend<L: ExtiLine>(line: L) {
        let line = line.raw_line();

        // Safety:
        // - We've ensured that the only 1-bit written is a valid line.
        // - This mirrors the `NVIC::pend` API and implementation, which is
        //   presumed safe.
        // - This is a "clear by writing 1" register, and this is a single write
        //   operation that cannot be interrupted.
        unsafe {
            (*Self::ptr()).pr.write(|w| w.bits(1 << line));
        }
    }

    /// Returns whether `line` is currently marked as pending.
    fn is_pending<L: ExtiLine>(line: L) -> bool {
        let bm: u32 = 1 << line.raw_line();

        // Safety: This is a read without side effects that cannot be
        // interrupted.
        let pr = unsafe { (*Self::ptr()).pr.read().bits() };

        pr & bm != 0
    }

    /// Enters a low-power mode until an interrupt occurs.
    ///
    /// Please note that this method will return after _any_ interrupt that can
    /// wake up the microcontroller from the given power mode.
    ///
    /// # Panics
    ///
    /// Panics, if `line` is an invalid EXTI line (reserved or not defined).
    /// Check the Reference Manual for a list of valid lines.
    fn wait_for_irq<L, M>(&mut self, line: L, mut power_mode: M)
        where L: ExtiLine, M: PowerMode,
    {
        let interrupt = line.interrupt();

        // This construct allows us to wait for the interrupt without having to
        // define an interrupt handler.
        interrupt::free(|_| {
            unsafe { NVIC::unmask(interrupt); }

            power_mode.enter();

            Self::unpend(line);
            NVIC::unpend(interrupt);
            NVIC::mask(interrupt);
        });
    }
}
