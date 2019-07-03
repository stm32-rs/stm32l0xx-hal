//! External interrupt controller
use crate::bb;
use crate::gpio;
use crate::pac::EXTI;
use crate::rcc;
use crate::rcc::Rcc;

#[cfg(feature = "stm32l0x1")]
use stm32l0::stm32l0x1::SYSCFG as syscfg_comp;
#[cfg(feature = "stm32l0x2")]
use stm32l0::stm32l0x2::SYSCFG_COMP as syscfg_comp;

pub enum Interrupt {
    exti0_1,
    exti2_3,
    exti4_15,
}

pub enum TriggerEdge {
    Rising,
    Falling,
    All,
}

pub trait ExtiExt {
    fn listen(
        &self,
        rcc: &mut Rcc,
        syscfg: &mut syscfg_comp,
        port: gpio::Port,
        line: u8,
        edge: TriggerEdge,
    );
    fn unlisten(&self, line: u8);
    fn pend_interrupt(&self, line: u8);
    fn clear_irq(&self, line: u8);
    fn get_pending_irq(&self) -> u32;
}

pub fn line_is_triggered(reg: u32, line: u8) -> bool {
    (reg & (0b1 << line)) != 0
}

impl ExtiExt for EXTI {
    fn listen(
        &self,
        rcc: &mut Rcc,
        syscfg: &mut syscfg_comp,
        port: gpio::Port,
        line: u8,
        edge: TriggerEdge,
    ) {
        #[cfg(feature = "stm32l0x1")]
        assert!(line < 24);
        #[cfg(feature = "stm32l0x2")]
        assert!(line < 15);

        // ensure that the SYSCFG peripheral is powered on
        // SYSCFG is necessary to change which PORT is routed to EXTIn
        rcc.enable(rcc::Peripheral::SYSCFG);

        // translate port into bit values for EXTIn registers
        let port_bm = match port {
            gpio::Port::PA => 0,
            gpio::Port::PB => 1,
            #[cfg(any(feature = "stm32l0x2"))]
            gpio::Port::PC => 2,
            #[cfg(any(feature = "stm32l0x2"))]
            gpio::Port::PD => 3,
            #[cfg(any(feature = "stm32l0x2"))]
            gpio::Port::PH => 7,
        };

        unsafe {
            match line {
                0 | 1 | 2 | 3 => {
                    syscfg.exticr1.write(|w| match line {
                        0 => w.exti0().bits(port_bm),
                        1 => w.exti1().bits(port_bm),
                        2 => w.exti2().bits(port_bm),
                        3 => w.exti3().bits(port_bm),
                        _ => w,
                    });
                }
                4 | 5 | 6 | 7 => {
                    syscfg.exticr2.write(|w| match line {
                        4 => w.exti4().bits(port_bm),
                        5 => w.exti5().bits(port_bm),
                        6 => w.exti6().bits(port_bm),
                        7 => w.exti7().bits(port_bm),
                        _ => w,
                    });
                }
                8 | 9 | 10 | 11 => {
                    syscfg.exticr3.write(|w| match line {
                        8 => w.exti8().bits(port_bm),
                        9 => w.exti9().bits(port_bm),
                        10 => w.exti10().bits(port_bm),
                        11 => w.exti11().bits(port_bm),
                        _ => w,
                    });
                }
                12 | 13 | 14 | 15 => {
                    syscfg.exticr4.write(|w| match line {
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

        let bm: u32 = 0b1 << line;

        unsafe {
            match edge {
                TriggerEdge::Rising => self.rtsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Falling => self.ftsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::All => {
                    self.rtsr.modify(|r, w| w.bits(r.bits() | bm));
                    self.ftsr.modify(|r, w| w.bits(r.bits() | bm));
                }
            }

            self.imr.modify(|r, w| w.bits(r.bits() | bm));
        }
    }

    fn unlisten(&self, line: u8) {
        assert!(line < 24);
        bb::clear(&self.rtsr, line);
        bb::clear(&self.ftsr, line);
        bb::clear(&self.imr, line);
    }

    fn pend_interrupt(&self, line: u8) {
        assert!(line < 24);
        bb::set(&self.swier, line);
    }

    fn get_pending_irq(&self) -> u32 {
        self.pr.read().bits()
    }

    fn clear_irq(&self, line: u8) {
        assert!(line < 24);

        self.pr.modify(|_, w| unsafe { w.bits(0b1 << line) });
    }
}
