pub use embedded_hal::prelude::*;

pub use crate::hal::adc::OneShot as _hal_adc_OneShot;
pub use crate::hal::watchdog::Watchdog as _hal_watchdog_Watchdog;
pub use crate::hal::watchdog::WatchdogEnable as _hal_watchdog_WatchdogEnable;

pub use crate::adc::AdcExt as _stm32l0xx_hal_analog_AdcExt;
pub use crate::delay::DelayExt as _stm32l0xx_hal_delay_DelayExt;
pub use crate::exti::ExtiExt as _stm32l0xx_hal_exti_ExtiExt;
pub use crate::gpio::GpioExt as _stm32l0xx_hal_gpio_GpioExt;
pub use crate::i2c::I2c1Ext as _stm32l0xx_hal_i2c1_I2cExt;
pub use crate::pwm::PwmExt as _stm32l0xx_hal_pwm_PwmExt;
pub use crate::rcc::RccExt as _stm32l0xx_hal_rcc_RccExt;
pub use crate::serial::Serial1Ext as _stm32l0xx_hal_serial_Serial1Ext;
pub use crate::serial::Serial2Ext as _stm32l0xx_hal_serial_Serial2Ext;
pub use crate::time::MonoTimerExt as _stm32l0xx_hal_time_MonoTimerExt;
pub use crate::time::U32Ext as _stm32l0xx_hal_time_U32Ext;
pub use crate::timer::TimerExt as _stm32l0xx_hal_timer_TimerExt;
pub use crate::watchdog::IndependedWatchdogExt as _stm32l0xx_hal_watchdog_IndependedWatchdogExt;
pub use crate::watchdog::WindowWatchdogExt as _stm32l0xx_hal_watchdog_WindowWatchdogExt;
