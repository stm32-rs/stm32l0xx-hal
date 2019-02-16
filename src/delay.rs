//! Delays
use crate::rcc::Clocks;
use crate::time::MicroSeconds;
use cast::u32;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SYST;
use hal::blocking::delay::{DelayMs, DelayUs};

pub trait DelayExt {
    fn delay(self, clocks: Clocks) -> Delay;
}

impl DelayExt for SYST {
    fn delay(self, clocks: Clocks) -> Delay {
        Delay::new(self, clocks)
    }
}

/// System timer (SysTick) as a delay provider
pub struct Delay {
    ticks_per_us: u32,
    syst: SYST,
}

impl Delay {
    /// Configures the system timer (SysTick) as a delay provider
    pub fn new(mut syst: SYST, clocks: Clocks) -> Self {
        syst.set_clock_source(SystClkSource::Core);
        let freq = clocks.sys_clk().0;
        assert!(freq > 1_000_000_u32);
        let ticks_per_us = freq / 1_000_000_u32;
        Delay { syst, ticks_per_us }
    }
    pub fn delay<T>(&mut self, delay: T)
    where
        T: Into<MicroSeconds>,
    {
        self.delay_us(delay.into().0)
    }

    /// Releases the system timer (SysTick) resource
    pub fn free(self) -> SYST {
        self.syst
    }
}

impl DelayMs<u32> for Delay {
    fn delay_ms(&mut self, ms: u32) {
        self.delay_us(ms * 1_000);
    }
}

impl DelayMs<u16> for Delay {
    fn delay_ms(&mut self, ms: u16) {
        self.delay_ms(u32(ms));
    }
}

impl DelayMs<u8> for Delay {
    fn delay_ms(&mut self, ms: u8) {
        self.delay_ms(u32(ms));
    }
}

impl DelayUs<u32> for Delay {
    fn delay_us(&mut self, us: u32) {
        const MAX_RVR: u32 = 0x00FF_FFFF;
        let mut total_rvr = self.ticks_per_us * us;
        while total_rvr > 0 {
            let current_rvr = if total_rvr <= MAX_RVR {
                total_rvr
            } else {
                MAX_RVR
            };
            self.syst.set_reload(current_rvr);
            self.syst.clear_current();
            self.syst.enable_counter();
            total_rvr -= current_rvr;
            while !self.syst.has_wrapped() {}
            self.syst.disable_counter();
        }
    }
}

impl DelayUs<u16> for Delay {
    fn delay_us(&mut self, us: u16) {
        self.delay_us(u32(us))
    }
}

impl DelayUs<u8> for Delay {
    fn delay_us(&mut self, us: u8) {
        self.delay_us(u32(us))
    }
}
