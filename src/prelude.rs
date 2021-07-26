pub use embedded_hal::{
    adc::OneShot as _,
    digital::v2::*,
    prelude::*,
    timer::Cancel as _,
    watchdog::{Watchdog as _, WatchdogEnable as _},
};

pub use crate::{
    adc::AdcExt as _,
    crc::CrcExt as _stm32l0_hal_CrcExt,
    delay::DelayExt as _,
    encoder::{EncoderExt, PinCh1, PinCh2, Pins},
    gpio::GpioExt as _,
    pwr::PowerMode as _,
    rcc::RccExt as _,
    spi::SpiExt as _,
    timer::TimerExt as _,
    watchdog::{IndependedWatchdogExt as _, WindowWatchdogExt as _},
};

// Make items like `.Hz()`, `.microseconds()` available from embedded-time
pub use embedded_time::{
    duration::Extensions as DurationExtensions, rate::Extensions as RateExtensions,
};

#[cfg(any(
    feature = "io-STM32L021",
    feature = "io-STM32L031",
    feature = "io-STM32L051",
    feature = "io-STM32L071",
))]
pub use crate::i2c::I2cExt as _;

#[cfg(any(feature = "io-STM32L051", feature = "io-STM32L071",))]
pub use crate::serial::{Serial1Ext as _, Serial1LpExt as _};
#[cfg(any(
    feature = "io-STM32L021",
    feature = "io-STM32L031",
    feature = "io-STM32L051",
    feature = "io-STM32L071",
))]
pub use crate::serial::{Serial1LpExt as _, Serial2Ext as _};
#[cfg(any(feature = "io-STM32L071",))]
pub use crate::serial::{Serial4Ext as _, Serial5Ext as _};
