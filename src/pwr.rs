//! Interface to the Power control (PWR) peripheral
//!
//! See STM32L0x2 reference manual, chapter 6.


use cortex_m::{
    asm,
    peripheral::SCB,
};

use crate::{
    pac,
    rcc::{
        ClockSrc,
        PLLSource,
        Rcc,
    },
};


/// Entry point to the PWR API
pub struct PWR(pac::PWR);

impl PWR {
    /// Create an instance of the PWR API
    pub fn new(pwr: pac::PWR, rcc: &mut Rcc) -> Self {
        // Reset peripheral
        rcc.rb.apb1rstr.modify(|_, w| w.pwrrst().set_bit());
        rcc.rb.apb1rstr.modify(|_, w| w.pwrrst().clear_bit());

        // Enable peripheral clock
        rcc.rb.apb1enr.modify(|_, w| w.pwren().set_bit());

        Self(pwr)
    }

    /// Returns a struct that can be used to enter Sleep mode
    pub fn sleep_mode<'r>(&'r mut self, scb: &'r mut SCB) -> SleepMode<'r> {
        SleepMode {
            scb,
        }
    }

    /// Returns a struct that can be used to enter Stop mode
    pub fn stop_mode<'r>(&'r mut self,
        scb:    &'r mut SCB,
        rcc:    &'r mut Rcc,
        config: StopModeConfig,
    )
        -> StopMode<'r>
    {
        StopMode {
            pwr: &mut self.0,
            scb,
            rcc,
            config,
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
    scb: &'r mut SCB,
}

impl PowerMode for SleepMode<'_> {
    fn enter(&mut self) {
        self.scb.clear_sleepdeep();
        asm::wfi();
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
    pwr:    &'r mut pac::PWR,
    scb:    &'r mut SCB,
    rcc:    &'r mut Rcc,
    config: StopModeConfig,
}

impl PowerMode for StopMode<'_> {
    fn enter(&mut self) {
        self.scb.set_sleepdeep();

        // Restore current clock source after waking up from Stop mode.
        self.rcc.rb.cfgr.modify(|_, w|
            match self.rcc.clocks.source() {
                ClockSrc::MSI(_) =>
                    // Use MSI as clock source after wake-up
                    w.stopwuck().clear_bit(),
                ClockSrc::HSI16 | ClockSrc::PLL(PLLSource::HSI16, _, _) =>
                    // Use HSI16 as clock source after wake-up
                    w.stopwuck().set_bit(),
                _ =>
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
                    panic!("External clock not supported for Stop mode"),
            }
        );

        self.pwr.cr.modify(|_, w|
            w
                // Ultra-low-power mode
                .ulp().bit(self.config.ultra_low_power)
                // Enter Stop mode
                .pdds().stop_mode()
                // Disable internal voltage regulator
                .lpds().set_bit()
        );
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
