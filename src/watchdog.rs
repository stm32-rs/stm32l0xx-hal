use crate::rcc::Rcc;
use crate::stm32::{IWDG, WWDG};
use crate::time::Hertz;
use hal::watchdog;

pub struct IndependedWatchdog {
    iwdg: IWDG,
}

impl IndependedWatchdog {
    pub fn set_config(&mut self, pre: u8, reload: u16) {
        self.iwdg.kr.write(|w| w.key().reset());
        self.iwdg.kr.write(|w| w.key().enable());

        while self.iwdg.sr.read().pvu().bit() {}
        self.iwdg.pr.write(|w| w.pr().bits(pre));

        while self.iwdg.sr.read().rvu().bit() {}
        self.iwdg.rlr.write(|w| w.rl().bits(reload));

        self.iwdg.kr.write(|w| w.key().start());
        self.iwdg.kr.write(|w| w.key().reset());
    }
}

impl watchdog::Watchdog for IndependedWatchdog {
    fn feed(&mut self) {
        self.iwdg.kr.write(|w| w.key().reset());
    }
}

impl watchdog::WatchdogEnable for IndependedWatchdog {
    type Time = Hertz;

    fn start<T>(&mut self, period: T)
    where
        T: Into<Hertz>,
    {
        const LSI_CLOCK: u32 = 38_000_u32;

        let freq = period.into().0;
        let mut timeout = LSI_CLOCK / freq / 4;
        let mut pre = 0;
        let mut reload = 0;
        while pre < 7 {
            reload = timeout;
            if reload <= 0xFFF {
                break;
            }
            pre += 1;
            timeout /= 2;
        }
        self.set_config(pre, reload as u16);
    }
}

pub trait IndependedWatchdogExt {
    fn watchdog(self) -> IndependedWatchdog;
}

impl IndependedWatchdogExt for IWDG {
    fn watchdog(self) -> IndependedWatchdog {
        IndependedWatchdog { iwdg: self }
    }
}

pub struct WindowWatchdog {
    wwdg: WWDG,
    clk: u32,
}

impl watchdog::Watchdog for WindowWatchdog {
    fn feed(&mut self) {
        self.wwdg.cr.write(|w| w.t().bits(0xFF));
    }
}

impl WindowWatchdog {
    pub fn set_window<T>(&mut self, window: T)
    where
        T: Into<Hertz>,
    {
        let freq = window.into().0;
        let mut ticks = self.clk / freq;
        let mut pre = 0;
        let mut threshold = 0;
        while pre < 3 {
            threshold = ticks;
            if threshold <= 0x7F {
                break;
            }
            pre += 1;
            ticks /= 2;
        }

        let window_bits = if threshold >= 0xFF {
            0
        } else {
            0xFF - threshold as u8
        };
        self.wwdg
            .cfr
            .write(|w| w.wdgtb().bits(pre).w().bits(window_bits));
    }

    pub fn listen(&mut self) {
        self.wwdg.cfr.write(|w| w.ewi().set_bit());
    }
}

impl watchdog::WatchdogEnable for WindowWatchdog {
    type Time = Hertz;

    fn start<T>(&mut self, period: T)
    where
        T: Into<Hertz>,
    {
        self.set_window(period);
        self.wwdg.cr.write(|w| w.wdga().set_bit().t().bits(0xFF));
    }
}

pub trait WindowWatchdogExt {
    fn watchdog(self, rcc: &mut Rcc) -> WindowWatchdog;
}

impl WindowWatchdogExt for WWDG {
    fn watchdog(self, rcc: &mut Rcc) -> WindowWatchdog {
        rcc.rb.apb1enr.modify(|_, w| w.wwdgen().set_bit());
        WindowWatchdog {
            wwdg: self,
            clk: rcc.clocks.apb1_clk().0 / 4096,
        }
    }
}
