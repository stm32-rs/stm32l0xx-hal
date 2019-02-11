use cortex_m::peripheral::DWT;

#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub struct Bps(pub u32);

#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub struct Hertz(pub u32);

#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub struct MicroSeconds(pub u32);

/// Extension trait that adds convenience methods to the `u32` type
pub trait U32Ext {
    /// Wrap in `Bps`
    fn bps(self) -> Bps;

    /// Wrap in `Hertz`
    fn hz(self) -> Hertz;

    /// Wrap in `Hertz`
    fn khz(self) -> Hertz;

    /// Wrap in `Hertz`
    fn mhz(self) -> Hertz;

    /// Wrap in `MicroSeconds`
    fn us(self) -> MicroSeconds;

    /// Wrap in `MicroSeconds`
    fn ms(self) -> MicroSeconds;
}

pub trait MonoTimerExt {
    fn monotonic<T>(self, sys_clk: T) -> MonoTimer
    where
        T: Into<Hertz>;
}

impl U32Ext for u32 {
    fn bps(self) -> Bps {
        Bps(self)
    }

    fn hz(self) -> Hertz {
        Hertz(self)
    }

    fn khz(self) -> Hertz {
        Hertz(self * 1_000)
    }

    fn mhz(self) -> Hertz {
        Hertz(self * 1_000_000)
    }

    fn ms(self) -> MicroSeconds {
        MicroSeconds(self * 1_000)
    }

    fn us(self) -> MicroSeconds {
        MicroSeconds(self)
    }
}

impl Into<MicroSeconds> for Hertz {
    fn into(self) -> MicroSeconds {
        let freq = self.0;
        assert!(freq != 0 && freq <= 1_000_000);
        MicroSeconds(1_000_000 / freq)
    }
}

impl Into<Hertz> for MicroSeconds {
    fn into(self) -> Hertz {
        let period = self.0;
        assert!(period != 0 && period <= 1_000_000);
        Hertz(1_000_000 / period)
    }
}

/// A monotonic nondecreasing timer
#[derive(Clone, Copy)]
pub struct MonoTimer {
    frequency: Hertz,
}

impl MonoTimer {
    /// Creates a new `Monotonic` timer
    pub fn new<T>(mut dwt: DWT, sys_clk: T) -> Self
    where
        T: Into<Hertz>,
    {
        dwt.enable_cycle_counter();

        // now the CYCCNT counter can't be stopped or resetted
        drop(dwt);

        MonoTimer {
            frequency: sys_clk.into(),
        }
    }

    /// Returns the frequency at which the monotonic timer is operating at
    pub fn frequency(&self) -> Hertz {
        self.frequency
    }

    /// Returns an `Instant` corresponding to "now"
    pub fn now(&self) -> Instant {
        Instant {
            now: DWT::get_cycle_count(),
        }
    }
}

impl MonoTimerExt for DWT {
    fn monotonic<T>(self, sys_clk: T) -> MonoTimer
    where
        T: Into<Hertz>,
    {
        MonoTimer::new(self, sys_clk)
    }
}

/// A measurement of a monotonically nondecreasing clock
#[derive(Clone, Copy)]
pub struct Instant {
    now: u32,
}

impl Instant {
    /// Ticks elapsed since the `Instant` was created
    pub fn elapsed(&self) -> u32 {
        DWT::get_cycle_count().wrapping_sub(self.now)
    }
}
