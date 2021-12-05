//! Interface to the Power control (PWR) peripheral
//!
//! See STM32L0x2 reference manual, chapter 6.

use cortex_m::{asm, peripheral::SCB};

use crate::{
    pac,
    rcc::{ClockSrc, Clocks, Enable, PLLSource, Rcc},
};

/// Entry point to the PWR API
pub struct PWR(pac::PWR);

impl PWR {
    /// Create an instance of the PWR API
    pub fn new(pwr: pac::PWR, rcc: &mut Rcc) -> Self {
        // Peripheral is not being reset here. First, there's no type state here
        // that would require any specific configuration. Second, there are
        // specific requirements that make resetting the peripheral complicated.
        // See STM32L0x2 reference manual, section 6.4.1 (VOS field).

        // Enable peripheral clock
        pac::PWR::enable(rcc);

        // Disable backup write protection. This is required to access various
        // register of various peripherals, so don't remove this unless you know
        // what you're doing and also change the affected peripheral APIs
        // accordingly.
        pwr.cr.modify(|_, w| w.dbp().set_bit());

        Self(pwr)
    }

    /// Switch voltage range of internal regulator
    ///
    /// Please note that switching Vcore has consequences, so please make sure
    /// you know what you're doing. See STM32L0x2 reference manual, sections
    /// 6.1.3 and following.
    pub fn switch_vcore_range(&mut self, range: VcoreRange) {
        // The STM32L0x2 reference manual, section 6.1.5 describes the procedure
        // being followed here.

        while self.0.csr.read().vosf().bit_is_set() {}

        // Safe, as `VcoreRange` only provides valid bit patterns.
        self.0
            .cr
            .modify(|_, w| unsafe { w.vos().bits(range as u8) });

        while self.0.csr.read().vosf().bit_is_set() {}
    }

    /// Returns currently configured internal regulator voltage range
    pub fn get_vcore_range(&mut self) -> VcoreRange {
        let vos = self.0.cr.read().vos().bits();

        // Shouldn't panic, as reading the field from the register should always
        // return a valid value.
        VcoreRange::from_bits(vos)
    }

    /// Enters low-power run mode
    ///
    /// Please note that there are some restrictions placed on low-power run
    /// mode. Please refer to the STM32L0x2 reference manual, section 6.3.4 for
    /// more information.
    ///
    /// # Panics
    ///
    /// To enter low-power run mode, the system clock frequency should not
    /// exceed the MSI frequency range 1 (131.072 kHz). This method will panic,
    /// if that is the case.
    pub fn enter_low_power_run_mode(&mut self, clocks: Clocks) {
        // This follows the procedure laid out in the STM32L0x2 reference
        // manual, section 6.3.4.

        // Panic, if system clock frequency is outside of allowed range. See
        // STM32L0x1/STM32L0x2/STM32L0x3 reference manuals, sections 6.3.4 and
        // 7.2.3.
        assert!(clocks.sys_clk().0 <= 131_072);

        self.switch_vcore_range(VcoreRange::Range2);

        // First set LPSDSR, then LPRUN, to go into low-power run mode. See
        // STM32L0x2 reference manual, section 6.4.1.
        self.set_lpsdsr();
        self.0.cr.modify(|_, w| w.lprun().set_bit());
    }

    /// Exit low-power run mode
    ///
    /// Please note that entering low-power run mode sets Vcore to range 2. This
    /// method will not switch Vcore again, so please make sure to restore the
    /// previous Vcore setting again, if you want to do so. See
    /// [`PWR::switch_vcore_range`]/[`PRW::get_vcore_range`] for more info.
    pub fn exit_low_power_run_mode(&mut self) {
        // First reset LPRUN, then LPSDSR. See STM32L0x2 reference manual,
        // section 6.4.1.
        self.0.cr.modify(|_, w| w.lprun().clear_bit());
        self.clear_lpsdsr();
    }

    /// Returns a struct that can be used to enter Sleep mode
    pub fn sleep_mode<'r>(&'r mut self, scb: &'r mut SCB) -> SleepMode<'r> {
        SleepMode { pwr: self, scb }
    }

    /// Returns a struct that can be used to enter low-power sleep mode
    ///
    /// # Panics
    ///
    /// To enter low-power sleep mode, the system clock frequency should not
    /// exceed the MSI frequency range 1 (131.072 kHz). This method will panic,
    /// if that is the case.
    pub fn low_power_sleep_mode<'r>(
        &'r mut self,
        scb: &'r mut SCB,
        rcc: &mut Rcc,
    ) -> LowPowerSleepMode<'r> {
        // Panic, if system clock frequency is outside of allowed range. See
        // STM32L0x1/STM32L0x2/STM32L0x3 reference manuals, sections 6.3.8 and
        // 7.2.3.
        assert!(rcc.clocks.sys_clk().0 <= 131_072);

        LowPowerSleepMode { pwr: self, scb }
    }

    /// Returns a struct that can be used to enter Stop mode
    pub fn stop_mode<'r>(
        &'r mut self,
        scb: &'r mut SCB,
        rcc: &'r mut Rcc,
        config: StopModeConfig,
    ) -> StopMode<'r> {
        StopMode {
            pwr: self,
            scb,
            rcc,
            config,
        }
    }

    /// Returns a struct that can be used to enter Standby mode
    pub fn standby_mode<'r>(&'r mut self, scb: &'r mut SCB) -> StandbyMode<'r> {
        StandbyMode { pwr: self, scb }
    }

    /// Private method to set LPSDSR
    fn set_lpsdsr(&mut self) {
        self.0.cr.modify(|_, w| w.lpsdsr().low_power_mode());
    }

    /// Private method to clear LPSDSR
    fn clear_lpsdsr(&mut self) {
        self.0.cr.modify(|_, w| w.lpsdsr().main_mode());
    }
}

/// Voltage range selection for internal voltage regulator
///
/// Used as an argument for [`PWR::switch_vcore_range`].
#[repr(u8)]
pub enum VcoreRange {
    /// Range 1 (1.8 V)
    Range1 = 0b01,

    /// Range 2 (1.5 V)
    Range2 = 0b10,

    /// Range 3 (1.2 V)
    Range3 = 0b11,
}

impl VcoreRange {
    /// Creates a `VcoreRange` instance from a bit pattern
    ///
    /// # Panics
    ///
    /// Panics, if an invalid value is passed. See STM32L0x2 reference manual,
    /// section 6.4.1 (documentation of VOS field) for valid values.
    pub fn from_bits(bits: u8) -> Self {
        match bits {
            0b01 => VcoreRange::Range1,
            0b10 => VcoreRange::Range2,
            0b11 => VcoreRange::Range3,
            bits => panic!("Bits don't represent valud Vcore range: {}", bits),
        }
    }
}

/// Implemented for all low-power modes
pub trait PowerMode {
    /// Enters the low-power mode
    fn enter(&mut self);
}

/// Sleep mode
///
/// You can get an instance of this struct by calling [`PWR::sleep_mode`].
///
/// The `PowerMode` implementation of this type will block until something wakes
/// the microcontroller up again. Please make sure to configure an interrupt, or
/// it could block forever.
///
/// Please note that entering Sleep mode may change the SCB configuration.
pub struct SleepMode<'r> {
    pwr: &'r mut PWR,
    scb: &'r mut SCB,
}

impl PowerMode for SleepMode<'_> {
    fn enter(&mut self) {
        self.pwr.clear_lpsdsr();
        self.scb.clear_sleepdeep();

        asm::dsb();
        asm::wfi();
    }
}

/// Low-power sleep mode
///
/// You can get an instance of this struct by calling
/// [`PWR::low_power_sleep_mode`].
///
/// The `PowerMode` implementation of this type will block until something wakes
/// the microcontroller up again. Please make sure to configure an interrupt, or
/// it could block forever.
///
/// Please note that entering low-power sleep mode may change the SCB
/// configuration.
pub struct LowPowerSleepMode<'r> {
    pwr: &'r mut PWR,
    scb: &'r mut SCB,
}

impl PowerMode for LowPowerSleepMode<'_> {
    fn enter(&mut self) {
        // Switch Vcore to range 2. This is required to enter low-power sleep
        // mode, according to the reference manual, section 6.3.8.
        let old_vcore = self.pwr.get_vcore_range();
        self.pwr.switch_vcore_range(VcoreRange::Range2);

        self.pwr.set_lpsdsr();
        self.scb.clear_sleepdeep();

        asm::dsb();
        asm::wfi();

        // Switch back to previous voltage range.
        self.pwr.switch_vcore_range(old_vcore);
    }
}

/// Stop mode
///
/// You can get an instance of this struct by calling [`PWR::stop_mode`].
///
/// The `PowerMode` implementation of this type will block until something wakes
/// the microcontroller up again. Please make sure to configure an interrupt, or
/// it could block forever.
///
/// This method will always disable the internal voltage regulator during Stop
/// mode.
///
/// Please note that entering Stop mode may change the SCB configuration.
///
/// # Panics
///
/// Panics, if the external clock is selected as clock source. In principle, it
/// is possible to enter Stop mode with the external clock enabled, although
/// that might require special handling. This is explained in the STM32L0x2
/// Reference Manual, section 6.3.9.
pub struct StopMode<'r> {
    pwr: &'r mut PWR,
    scb: &'r mut SCB,
    rcc: &'r mut Rcc,
    config: StopModeConfig,
}

impl PowerMode for StopMode<'_> {
    fn enter(&mut self) {
        self.scb.set_sleepdeep();

        // Restore current clock source after waking up from Stop mode.
        self.rcc
            .rb
            .cfgr
            .modify(|_, w| match self.rcc.clocks.source() {
                // Use MSI as clock source after wake-up
                ClockSrc::MSI(_) => w.stopwuck().clear_bit(),
                // Use HSI16 as clock source after wake-up
                ClockSrc::HSI16(_) | ClockSrc::PLL(PLLSource::HSI16(_), _, _) => {
                    w.stopwuck().set_bit()
                }
                // External clock selected
                //
                // Unfortunately handling the external clock is not as
                // straight-forward as handling MSI or HSI16. We need to
                // know whether the external clock is going to be shut down
                // during Stop mode. If it is, we need to either shut it
                // down before entering Stop mode, or enable the clock
                // security system (CSS) and handle any failures using it.
                // This is explained in sectoin 6.3.9 of the STM32L0x2
                // Reference Manual.
                //
                // In principle, we could ask the user (through
                // `StopModeConfig`), whether to shut down the external
                // clock then restore is after we wake up again. However, to
                // do this we'd either need to refactor the `rcc` module,
                // making it more flexible so we can reuse the relevant code
                // here, or duplicate that code. I (hannobraun) am not to
                // keen on either right now, given that I don't have a test
                // setup with an external clock source at hand.
                //
                // One might ask why we need to restore the configuration at
                // all after waking up, but that's absolutely required. This
                // HAL's architecture assumes that the clocks are configured
                // once, then never changed again. If we left Stop mode with
                // a different clock frequency than we entered it with, a
                // lot of peripheral would stop working correctly.
                //
                // For now, I've decided to just not support this case and
                // panic, which is also documented in this method's doc
                // comment.
                _ => panic!("External clock not supported for Stop mode"),
            });

        // Configure Stop mode
        self.pwr.0.cr.modify(|_, w| {
            // Ultra-low-power mode
            w.ulp().bit(self.config.ultra_low_power);
            // Clear WUF
            w.cwuf().set_bit();
            // Enter Stop mode
            w.pdds().stop_mode();
            // Disable internal voltage regulator
            w.lpds().set_bit()
        });

        // Wait for WUF to be cleared
        while self.pwr.0.csr.read().wuf().bit_is_set() {}

        // Enter Stop mode
        asm::dsb();
        asm::wfi();
    }
}

/// Configuration for entering Stop mode
///
/// Used by `StopMode`'s `PowerMode` implementation.
pub struct StopModeConfig {
    /// Disable additional hardware when entering Stop mode
    ///
    /// When set to `true`, the following hardware will be disabled:
    ///
    /// - Internal voltage reference (Vrefint)
    /// - Brown out reset (BOR)
    /// - Programmable voltage detector (PVD)
    /// - Internal temperature sensor
    pub ultra_low_power: bool,
}

/// Standby mode
///
/// You can get an instance of this struct by calling [`PWR::standby_mode`].
///
/// The `PowerMode` implementation of this type will block until something wakes
/// the microcontroller up again. Please make sure to configure an interrupt, or
/// it could block forever. Once woken up, the method will not return. Instead,
/// the microcontroller will reset.
pub struct StandbyMode<'r> {
    pwr: &'r mut PWR,
    scb: &'r mut SCB,
}

impl PowerMode for StandbyMode<'_> {
    fn enter(&mut self) {
        // Configure Standby mode
        self.scb.set_sleepdeep();
        self.pwr.0.cr.modify(|_, w| {
            // Clear WUF
            w.cwuf().set_bit();
            // Standby mode
            w.pdds().standby_mode()
        });

        // Wait for WUF to be cleared
        while self.pwr.0.csr.read().wuf().bit_is_set() {}

        // Enter Standby mode
        asm::dsb();
        asm::wfi();
    }
}
