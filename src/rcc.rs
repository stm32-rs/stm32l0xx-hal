use crate::pac::RCC;
use crate::time::{Hertz, U32Ext};

/// System clock mux source
pub enum ClockSrc {
    MSI(MSIRange),
    PLL(PLLSource, PLLMul, PLLDiv),
    HSE(Hertz),
    HSI16,
}

/// MSI Range
#[derive(Clone, Copy)]
pub enum MSIRange {
    Range0 = 0,
    Range1 = 1,
    Range2 = 2,
    Range3 = 3,
    Range4 = 4,
    Range5 = 5,
    Range6 = 6,
}

impl Default for MSIRange {
    fn default() -> MSIRange {
        MSIRange::Range5
    }
}

/// PLL divider
#[derive(Clone, Copy)]
pub enum PLLDiv {
    Div2 = 1,
    Div3 = 2,
    Div4 = 3,
}

/// PLL multiplier
#[derive(Clone, Copy)]
pub enum PLLMul {
    Mul3 = 0,
    Mul4 = 1,
    Mul6 = 2,
    Mul8 = 3,
    Mul12 = 4,
    Mul16 = 5,
    Mul24 = 6,
    Mul32 = 7,
    Mul48 = 8,
}

/// AHB prescaler
#[derive(Clone, Copy)]
pub enum AHBPrescaler {
    NotDivided = 0,
    Div2 = 0b1000,
    Div4 = 0b1001,
    Div8 = 0b1010,
    Div16 = 0b1011,
    Div64 = 0b1100,
    Div128 = 0b1101,
    Div256 = 0b1110,
    Div512 = 0b1111,
}

/// APB prescaler
#[derive(Clone, Copy)]
pub enum APBPrescaler {
    NotDivided = 0,
    Div2 = 0b100,
    Div4 = 0b101,
    Div8 = 0b110,
    Div16 = 0b111,
}

/// PLL clock input source
#[derive(Clone, Copy)]
pub enum PLLSource {
    HSI16,
    HSE(Hertz),
}

/// HSI speed
pub const HSI_FREQ: u32 = 16_000_000;

/// Clocks configutation
pub struct Config {
    mux: ClockSrc,
    ahb_pre: AHBPrescaler,
    apb1_pre: APBPrescaler,
    apb2_pre: APBPrescaler,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            mux: ClockSrc::MSI(MSIRange::default()),
            ahb_pre: AHBPrescaler::NotDivided,
            apb1_pre: APBPrescaler::NotDivided,
            apb2_pre: APBPrescaler::NotDivided,
        }
    }
}

impl Config {
    pub fn clock_src(mut self, mux: ClockSrc) -> Self {
        self.mux = mux;
        self
    }

    pub fn ahb_pre(mut self, pre: AHBPrescaler) -> Self {
        self.ahb_pre = pre;
        self
    }

    pub fn apb1_pre(mut self, pre: APBPrescaler) -> Self {
        self.apb1_pre = pre;
        self
    }

    pub fn apb2_pre(mut self, pre: APBPrescaler) -> Self {
        self.apb2_pre = pre;
        self
    }

    pub fn hsi16() -> Config {
        Config {
            mux: ClockSrc::HSI16,
            ahb_pre: AHBPrescaler::NotDivided,
            apb1_pre: APBPrescaler::NotDivided,
            apb2_pre: APBPrescaler::NotDivided,
        }
    }

    pub fn msi(range: MSIRange) -> Config {
        Config {
            mux: ClockSrc::MSI(range),
            ahb_pre: AHBPrescaler::NotDivided,
            apb1_pre: APBPrescaler::NotDivided,
            apb2_pre: APBPrescaler::NotDivided,
        }
    }

    pub fn pll(pll_src: PLLSource, pll_mul: PLLMul, pll_div: PLLDiv) -> Config {
        Config {
            mux: ClockSrc::PLL(pll_src, pll_mul, pll_div),
            ahb_pre: AHBPrescaler::NotDivided,
            apb1_pre: APBPrescaler::NotDivided,
            apb2_pre: APBPrescaler::NotDivided,
        }
    }

    pub fn hse<T>(freq: T) -> Config
    where
        T: Into<Hertz>,
    {
        Config {
            mux: ClockSrc::HSE(freq.into()),
            ahb_pre: AHBPrescaler::NotDivided,
            apb1_pre: APBPrescaler::NotDivided,
            apb2_pre: APBPrescaler::NotDivided,
        }
    }
}

/// RCC peripheral
pub struct Rcc {
    pub clocks: Clocks,
    pub(crate) rb: RCC,
}

pub enum Peripheral {
    // CRS,
    // DAC,
    // I2C1,
    // I2C2,
    // I2C3,
    // LPTIME1,
    // LPUART1,
    // PWREN,
    // SPI2EN,
    // TIM2EN,
    // TIM3EN,
    // USART2,
    // USART4,
    // USART5,
    // USB,
    // ADC,
    // DBG,
    // MIFI,
    // SPI,
    SYSCFG,
    // TIM21,
    // TIM22,
    // UART1
}

impl Rcc {
    pub fn enable(&self, p: Peripheral) {
        match p {
            Peripheral::SYSCFG => {
                self.rb.apb2enr.modify(|_, w| w.syscfgen().bit(true));
            }
        }
    }
}

/// Extension trait that freezes the `RCC` peripheral with provided clocks configuration
pub trait RccExt {
    fn freeze(self, config: Config) -> Rcc;
}

impl RccExt for RCC {
    fn freeze(self, cfgr: Config) -> Rcc {
        let (sys_clk, sw_bits) = match cfgr.mux {
            ClockSrc::MSI(range) => {
                let range = range as u8;
                // Set MSI range
                #[cfg(feature = "stm32l0x1")]
                self.icscr.write(|w| w.msirange().bits(range));
                #[cfg(feature = "stm32l0x2")]
                self.icscr.write(|w| unsafe { w.msirange().bits(range) });

                // Enable MSI
                self.cr.write(|w| w.msion().set_bit());
                while self.cr.read().msirdy().bit_is_clear() {}

                let freq = 32_768 * (1 << (range + 1));
                (freq, 0)
            }
            ClockSrc::HSI16 => {
                // Enable HSI16
                self.cr.write(|w| w.hsi16on().set_bit());
                while self.cr.read().hsi16rdyf().bit_is_clear() {}

                (HSI_FREQ, 1)
            }
            ClockSrc::HSE(freq) => {
                // Enable HSE
                self.cr.write(|w| w.hseon().set_bit());
                while self.cr.read().hserdy().bit_is_clear() {}

                (freq.0, 2)
            }
            ClockSrc::PLL(src, mul, div) => {
                let (src_bit, freq) = match src {
                    PLLSource::HSE(freq) => {
                        // Enable HSE
                        self.cr.write(|w| w.hseon().set_bit());
                        while self.cr.read().hserdy().bit_is_clear() {}
                        (true, freq.0)
                    }
                    PLLSource::HSI16 => {
                        // Enable HSI
                        self.cr.write(|w| w.hsi16on().set_bit());
                        while self.cr.read().hsi16rdyf().bit_is_clear() {}
                        (false, HSI_FREQ)
                    }
                };

                // Disable PLL
                self.cr.write(|w| w.pllon().clear_bit());
                while self.cr.read().pllrdy().bit_is_set() {}

                let mul_bytes = mul as u8;
                let div_bytes = div as u8;

                let freq = match mul {
                    PLLMul::Mul3 => freq * 3,
                    PLLMul::Mul4 => freq * 4,
                    PLLMul::Mul6 => freq * 6,
                    PLLMul::Mul8 => freq * 8,
                    PLLMul::Mul12 => freq * 12,
                    PLLMul::Mul16 => freq * 16,
                    PLLMul::Mul24 => freq * 24,
                    PLLMul::Mul32 => freq * 32,
                    PLLMul::Mul48 => freq * 48,
                };

                let freq = match div {
                    PLLDiv::Div2 => freq / 2,
                    PLLDiv::Div3 => freq / 3,
                    PLLDiv::Div4 => freq / 4,
                };
                assert!(freq <= 24.mhz().0);

                self.cfgr.write(move |w| unsafe {
                    w.pllmul()
                        .bits(mul_bytes)
                        .plldiv()
                        .bits(div_bytes)
                        .pllsrc()
                        .bit(src_bit)
                });

                // Enable PLL
                self.cr.write(|w| w.pllon().set_bit());
                while self.cr.read().pllrdy().bit_is_clear() {}

                (freq, 3)
            }
        };

        self.cfgr.modify(|_, w| unsafe {
            w.sw()
                .bits(sw_bits)
                .hpre()
                .bits(cfgr.ahb_pre as u8)
                .ppre1()
                .bits(cfgr.apb1_pre as u8)
                .ppre2()
                .bits(cfgr.apb2_pre as u8)
        });

        let ahb_freq = match cfgr.ahb_pre {
            AHBPrescaler::NotDivided => sys_clk,
            pre => sys_clk / (1 << (pre as u8 - 7)),
        };

        let (apb1_freq, apb1_tim_freq) = match cfgr.apb1_pre {
            APBPrescaler::NotDivided => (ahb_freq, ahb_freq),
            pre => {
                let freq = ahb_freq / (1 << (pre as u8 - 3));
                (freq, freq * 2)
            }
        };

        let (apb2_freq, apb2_tim_freq) = match cfgr.apb2_pre {
            APBPrescaler::NotDivided => (ahb_freq, ahb_freq),
            pre => {
                let freq = ahb_freq / (1 << (pre as u8 - 3));
                (freq, freq * 2)
            }
        };

        let clocks = Clocks {
            sys_clk: sys_clk.hz(),
            ahb_clk: ahb_freq.hz(),
            apb1_clk: apb1_freq.hz(),
            apb2_clk: apb2_freq.hz(),
            apb1_tim_clk: apb1_tim_freq.hz(),
            apb2_tim_clk: apb2_tim_freq.hz(),
        };

        Rcc { rb: self, clocks }
    }
}

/// Frozen clock frequencies
///
/// The existence of this value indicates that the clock configuration can no longer be changed
#[derive(Clone, Copy)]
pub struct Clocks {
    sys_clk: Hertz,
    ahb_clk: Hertz,
    apb1_clk: Hertz,
    apb1_tim_clk: Hertz,
    apb2_clk: Hertz,
    apb2_tim_clk: Hertz,
}

impl Clocks {
    /// Returns the system (core) frequency
    pub fn sys_clk(&self) -> Hertz {
        self.sys_clk
    }

    /// Returns the frequency of the AHB
    pub fn ahb_clk(&self) -> Hertz {
        self.ahb_clk
    }

    /// Returns the frequency of the APB1
    pub fn apb1_clk(&self) -> Hertz {
        self.apb1_clk
    }

    /// Returns the frequency of the APB1 timers
    pub fn apb1_tim_clk(&self) -> Hertz {
        self.apb1_tim_clk
    }

    /// Returns the frequency of the APB2
    pub fn apb2_clk(&self) -> Hertz {
        self.apb2_clk
    }

    /// Returns the frequency of the APB2 timers
    pub fn apb2_tim_clk(&self) -> Hertz {
        self.apb2_tim_clk
    }
}
