use super::*;

macro_rules! bus_enable {
    ($PER:ident => $en:ident) => {
        impl Enable for crate::pac::$PER {
            #[inline(always)]
            fn enable(rcc: &mut Rcc) {
                Self::Bus::enr(rcc).modify(|_, w| w.$en().set_bit());
                cortex_m::asm::dsb();
            }
            #[inline(always)]
            fn disable(rcc: &mut Rcc) {
                Self::Bus::enr(rcc).modify(|_, w| w.$en().clear_bit());
            }
            #[inline(always)]
            fn is_enabled() -> bool {
                let rcc = unsafe { &*RCC::ptr() };
                Self::Bus::enr(rcc).read().$en().bit_is_set()
            }
            #[inline(always)]
            fn is_disabled() -> bool {
                let rcc = unsafe { &*RCC::ptr() };
                Self::Bus::enr(rcc).read().$en().bit_is_clear()
            }
            #[inline(always)]
            unsafe fn enable_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::enr(rcc).modify(|_, w| w.$en().set_bit());
                cortex_m::asm::dsb();
            }
            #[inline(always)]
            unsafe fn disable_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::enr(rcc).modify(|_, w| w.$en().clear_bit());
            }
        }
    };
}
macro_rules! bus_smenable {
    ($PER:ident => $smen:ident) => {
        impl SMEnable for crate::pac::$PER {
            #[inline(always)]
            fn enable_in_sleep_mode(rcc: &mut Rcc) {
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().set_bit());
                cortex_m::asm::dsb();
            }
            #[inline(always)]
            fn disable_in_sleep_mode(rcc: &mut Rcc) {
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().clear_bit());
            }
            #[inline(always)]
            fn is_enabled_in_sleep_mode() -> bool {
                let rcc = unsafe { &*RCC::ptr() };
                Self::Bus::smenr(rcc).read().$smen().bit_is_set()
            }
            #[inline(always)]
            fn is_disabled_in_sleep_mode() -> bool {
                let rcc = unsafe { &*RCC::ptr() };
                Self::Bus::smenr(rcc).read().$smen().bit_is_clear()
            }
            #[inline(always)]
            unsafe fn enable_in_sleep_mode_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().set_bit());
                cortex_m::asm::dsb();
            }
            #[inline(always)]
            unsafe fn disable_in_sleep_mode_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().clear_bit());
            }
        }
    };
}
macro_rules! bus_reset {
    ($PER:ident => $rst:ident) => {
        impl Reset for crate::pac::$PER {
            #[inline(always)]
            fn reset(rcc: &mut Rcc) {
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().set_bit());
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().clear_bit());
            }
            #[inline(always)]
            unsafe fn reset_unchecked() {
                let rcc = &*RCC::ptr();
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().set_bit());
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().clear_bit());
            }
        }
    };
}

macro_rules! bus {
    ($($PER:ident => ($busX:ty, $($en:ident)?, $($smen:ident)?, $($rst:ident)?),)+) => {
        $(
            impl crate::Sealed for crate::pac::$PER {}
            impl RccBus for crate::pac::$PER {
                type Bus = $busX;
            }
            $(bus_enable!($PER => $en);)?
            $(bus_smenable!($PER => $smen);)?
            $(bus_reset!($PER => $rst);)?
        )+
    }
}

bus! {
    GPIOA => (IOP, iopaen, iopasmen, ioparst), // 0
    GPIOB => (IOP, iopben, iopbsmen, iopbrst), // 1
    GPIOC => (IOP, iopcen, iopcsmen, iopcrst), // 2
    GPIOD => (IOP, iopden, iopdsmen, iopdrst), // 3
    GPIOE => (IOP, iopeen, iopesmen, ioperst), // 4
    GPIOH => (IOP, iophen, iophsmen, iophrst), // 7

    DMA1 => (AHB, dmaen, dmasmen, dmarst), // 0
    FLASH => (AHB, mifen, mifsmen, mifrst), // 8
    CRC => (AHB, crcen, crcsmen, crcrst), // 12
    AES => (AHB, crypen, crypsmen, cryprst), // 24

    TIM2 => (APB1, tim2en, tim2smen, tim2rst), // 0
    TIM6 => (APB1, tim6en, tim6smen, tim6rst), // 4
    TIM7 => (APB1, tim7en, tim7smen, tim7rst), // 5
    SPI2 => (APB1, spi2en, spi2smen, spi2rst), // 14
    LPUART1 => (APB1, lpuart1en, lpuart1smen, lpuart1rst), // 20
    USART4 => (APB1, usart4en, usart4smen, usart4rst), // 19
    USART5 => (APB1, usart5en, usart5smen, usart5rst), // 20
    I2C1 => (APB1, i2c1en, i2c1smen, i2c1rst), // 21
    I2C2 => (APB1, i2c2en, i2c2smen, i2c2rst), // 22
    PWR => (APB1, pwren, pwrsmen, pwrrst), // 28
    I2C3 => (APB1, i2c3en, i2c3smen, i2c3rst), // 30
    LPTIM => (APB1, lptim1en, lptim1smen, lptim1rst), // 31

    SYSCFG => (APB2, syscfgen, syscfgsmen, syscfgrst), // 0
    TIM21 => (APB2, tim21en, tim21smen, tim21rst), // 2
    TIM22 => (APB2, tim22en, tim22smen, tim22rst), // 5
    ADC => (APB2, adcen, adcsmen, adcrst), // 9
    SPI1 => (APB2, spi1en, spi1smen, spi1rst), // 12
    USART1 => (APB2, usart1en, usart1smen, usart1rst), // 14
    DBG => (APB2, dbgen, dbgsmen, dbgrst), // 22
}

#[cfg(any(feature = "stm32l0x0", feature = "stm32l0x1"))]
bus! {
    WWDG => (APB1, wwdgen, wwdgsmen, wwdgrst), // 11
    USART2 => (APB1, usart2en, usart2smen, usart2rst), // 17

    FW => (APB2, fwen,,), // 7
}

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
bus! {
    TSC => (AHB, touchen, touchsmen, touchrst), // 16
    RNG => (AHB, rngen, rngsmen, rngrst), // 20

    WWDG => (APB1, wwdgen, wwdgsmen, wwdrst), // 11 // TODO: fix typo
    USART2 => (APB1, usart2en, usart2smen, lpuart12rst), // 17 // TODO: fix typo

    USB => (APB1, usben, usbsmen, usbrst), // 23
    CRS => (APB1, crsen, crssmen, crsrst), // 27
    DAC => (APB1, dacen, dacsmen, dacrst), // 29

    FW => (APB2, mifien,,), // 7
}

#[cfg(any(feature = "stm32l0x1", feature = "stm32l0x2", feature = "stm32l0x3"))]
bus! {
    TIM3 => (APB1, tim3en, tim3smen, tim3rst), // 1
}
