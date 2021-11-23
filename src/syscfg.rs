//! Interface to the SYSCFG peripheral
//!
//! See STM32L0x2 reference manual, chapter 10.

use crate::{
    pac,
    rcc::{Enable, Rcc, Reset},
};

type PacSyscfg = pac::SYSCFG;

pub struct SYSCFG {
    pub(crate) syscfg: PacSyscfg,
}

impl SYSCFG {
    pub fn new(syscfg: PacSyscfg, rcc: &mut Rcc) -> Self {
        // Enable SYSCFG peripheral
        PacSyscfg::enable(rcc);
        // Reset SYSCFG peripheral
        PacSyscfg::reset(rcc);

        SYSCFG { syscfg }
    }
}
