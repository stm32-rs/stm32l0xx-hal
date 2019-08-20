//! Interface to the SYSCFG peripheral
//!
//! See STM32L0x2 reference manual, chapter 10.


use crate::pac;


#[cfg(feature = "stm32l0x1")]
type PacSyscfg = pac::SYSCFG;

#[cfg(feature = "stm32l0x2")]
type PacSyscfg = pac::SYSCFG_COMP;


pub struct SYSCFG {
    pub(crate) syscfg: PacSyscfg,
}

impl SYSCFG {
    pub fn new(syscfg: PacSyscfg) -> Self {
        SYSCFG {
            syscfg
        }
    }
}
