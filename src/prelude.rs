pub use embedded_hal::digital::v2::*;
pub use embedded_hal::prelude::*;

pub use crate::hal::adc::OneShot as _;
pub use crate::hal::watchdog::Watchdog as _;
pub use crate::hal::watchdog::WatchdogEnable as _;

pub use crate::adc::AdcExt as _;
pub use crate::delay::DelayExt as _;
pub use crate::exti::ExtiExt as _;
pub use crate::gpio::GpioExt as _;
pub use crate::i2c::I2cExt as _;
pub use crate::rcc::RccExt as _;
pub use crate::serial::Serial1Ext as _;
pub use crate::serial::Serial2Ext as _;
pub use crate::spi::SpiExt as _;
pub use crate::time::U32Ext as _;
pub use crate::timer::TimerExt as _;
pub use crate::watchdog::IndependedWatchdogExt as _;
pub use crate::watchdog::WindowWatchdogExt as _;
