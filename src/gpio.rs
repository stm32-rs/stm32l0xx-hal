//! General Purpose Input / Output

use core::convert::Infallible;
use core::marker::PhantomData;

use crate::rcc::Rcc;

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The parts to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self, rcc: &mut Rcc) -> Self::Parts;
}

trait GpioRegExt {
    fn is_low(&self, pos: u8) -> bool;
    fn is_set_low(&self, pos: u8) -> bool;
    fn set_high(&self, pos: u8);
    fn set_low(&self, pos: u8);
}

/// Input mode (type state)
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Floating input (type state)
pub struct Floating;

/// Pulled down input (type state)
pub struct PullDown;

/// Pulled up input (type state)
pub struct PullUp;

/// Open drain input or output (type state)
pub struct OpenDrain;

/// Analog mode (type state)
pub struct Analog;

/// Output mode (type state)
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Push pull output (type state)
pub struct PushPull;

use embedded_hal::digital::v2::{toggleable, InputPin, OutputPin, StatefulOutputPin};

/// Fully erased pin
pub struct Pin<MODE> {
    i: u8,
    port: *const dyn GpioRegExt,
    _mode: PhantomData<MODE>,
}

// NOTE(unsafe) The only write acess is to BSRR, which is thread safe
unsafe impl<MODE> Sync for Pin<MODE> {}
// NOTE(unsafe) this only enables read access to the same pin from multiple
// threads
unsafe impl<MODE> Send for Pin<MODE> {}

impl<MODE> StatefulOutputPin for Pin<Output<MODE>> {
    #[inline(always)]
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        self.is_set_low().map(|v| !v)
    }

    #[inline(always)]
    fn is_set_low(&self) -> Result<bool, Self::Error> {
        Ok(unsafe { (*self.port).is_set_low(self.i) })
    }
}

impl<MODE> OutputPin for Pin<Output<MODE>> {
    type Error = Infallible;

    #[inline(always)]
    fn set_high(&mut self) -> Result<(), Self::Error> {
        unsafe { (*self.port).set_high(self.i) };
        Ok(())
    }

    #[inline(always)]
    fn set_low(&mut self) -> Result<(), Self::Error> {
        unsafe { (*self.port).set_low(self.i) }
        Ok(())
    }
}

impl<MODE> toggleable::Default for Pin<Output<MODE>> {}

impl InputPin for Pin<Output<OpenDrain>> {
    type Error = Infallible;

    #[inline(always)]
    fn is_high(&self) -> Result<bool, Self::Error> {
        self.is_low().map(|v| !v)
    }

    #[inline(always)]
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(unsafe { (*self.port).is_low(self.i) })
    }
}

impl<MODE> InputPin for Pin<Input<MODE>> {
    type Error = Infallible;

    #[inline(always)]
    fn is_high(&self) -> Result<bool, Self::Error> {
        self.is_low().map(|v| !v)
    }

    #[inline(always)]
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(unsafe { (*self.port).is_low(self.i) })
    }
}

mod sealed {
    pub trait Sealed {}
}

/// Marker trait for valid pin modes (type state).
///
/// It can not be implemented by outside types.
pub trait PinMode: sealed::Sealed {
    // These constants are used to implement the pin configuration code.
    // They are not part of public API.

    #[doc(hidden)]
    const PUPDR: u8;
    #[doc(hidden)]
    const MODER: u8;
    #[doc(hidden)]
    const OTYPER: Option<u8> = None;
}

impl sealed::Sealed for Input<Floating> {}
impl PinMode for Input<Floating> {
    const PUPDR: u8 = 0b00;
    const MODER: u8 = 0b00;
}

impl sealed::Sealed for Input<PullDown> {}
impl PinMode for Input<PullDown> {
    const PUPDR: u8 = 0b10;
    const MODER: u8 = 0b00;
}

impl sealed::Sealed for Input<PullUp> {}
impl PinMode for Input<PullUp> {
    const PUPDR: u8 = 0b01;
    const MODER: u8 = 0b00;
}

impl sealed::Sealed for Analog {}
impl PinMode for Analog {
    const PUPDR: u8 = 0b00;
    const MODER: u8 = 0b11;
}

impl sealed::Sealed for Output<OpenDrain> {}
impl PinMode for Output<OpenDrain> {
    const PUPDR: u8 = 0b00;
    const MODER: u8 = 0b01;
    const OTYPER: Option<u8> = Some(0b1);
}

impl sealed::Sealed for Output<PushPull> {}
impl PinMode for Output<PushPull> {
    const PUPDR: u8 = 0b00;
    const MODER: u8 = 0b01;
    const OTYPER: Option<u8> = Some(0b0);
}

/// GPIO Pin speed selection
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Speed {
    Low = 0,
    Medium = 1,
    High = 2,
    VeryHigh = 3,
}

#[allow(dead_code)]
pub enum AltMode {
    AF0 = 0,
    AF1 = 1,
    AF2 = 2,
    AF3 = 3,
    AF4 = 4,
    AF5 = 5,
    AF6 = 6,
    AF7 = 7,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Port {
    PA,
    PB,
    PC,
    PD,
    PE,
    PH,
}

macro_rules! gpio_trait {
    ($gpiox:ident) => {
        impl GpioRegExt for crate::pac::$gpiox::RegisterBlock {
            fn is_low(&self, pos: u8) -> bool {
                self.idr.read().bits() & (1 << pos) == 0
            }

            fn is_set_low(&self, pos: u8) -> bool {
                self.odr.read().bits() & (1 << pos) == 0
            }

            fn set_high(&self, pos: u8) {
                // NOTE(unsafe) atomic write to a stateless register
                unsafe { self.bsrr.write(|w| w.bits(1 << pos)) }
            }

            fn set_low(&self, pos: u8) {
                // NOTE(unsafe) atomic write to a stateless register
                unsafe { self.bsrr.write(|w| w.bits(1 << (pos + 16))) }
            }
        }
    };
}

gpio_trait!(gpioa);
gpio_trait!(gpiob);

macro_rules! gpio {
    ($GPIOX:ident, $gpiox:ident, $PXx:ident, [
        $($PXi:ident: ($pxi:ident, $i:expr, $MODE:ty),)+
    ]) => {
        /// GPIO
        pub mod $gpiox {
            use core::marker::PhantomData;

            use crate::hal::digital::v2::{toggleable, InputPin, OutputPin, StatefulOutputPin};
            use crate::pac::$GPIOX;
            use crate::rcc::{Enable, Rcc};
            use super::{
                Floating, GpioExt, Input, OpenDrain, Output, Speed,
                PullDown, PullUp, PushPull, AltMode, Analog, Port,
                PinMode, Pin, GpioRegExt
            };

            /// GPIO parts
            pub struct Parts {
                $(
                    /// Pin
                    pub $pxi: $PXi<$MODE>,
                )+
            }

            impl GpioExt for $GPIOX {
                type Parts = Parts;

                fn split(self, rcc: &mut Rcc) -> Parts {
                    <$GPIOX>::enable(rcc);

                    Parts {
                        $(
                            $pxi: $PXi {
                                _mode: PhantomData,
                            },
                        )+
                    }
                }
            }

            $(
                /// Pin
                pub struct $PXi<MODE> {
                    _mode: PhantomData<MODE>,
                }

                impl<MODE> $PXi<MODE> {
                    /// The port this pin is part of.
                    pub const PORT: Port = Port::$PXx;

                    /// The pin's number inside its port.
                    pub const PIN_NUMBER: u8 = $i;

                    /// Returns the port this pin is part of.
                    pub fn port(&self) -> Port {
                        Port::$PXx
                    }

                    /// Returns this pin's number inside its port.
                    pub fn pin_number(&self) -> u8 {
                        $i
                    }
                }

                impl<MODE: PinMode> $PXi<MODE> {
                    /// Puts `self` into mode `M`.
                    ///
                    /// This violates the type state constraints from `MODE`, so callers must
                    /// ensure they use this properly.
                    fn mode<M: PinMode>(&mut self) {
                        let offset = 2 * $i;
                        unsafe {
                            (*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (u32::from(M::PUPDR) << offset))
                            });

                            if let Some(otyper) = M::OTYPER {
                                (*$GPIOX::ptr()).otyper.modify(|r, w| {
                                    w.bits(r.bits() & !(0b1 << $i) | (u32::from(otyper) << $i))
                                });
                            }

                            (*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (u32::from(M::MODER) << offset))
                            });
                        }
                    }

                    fn with_mode<M, F, R>(
                        &mut self,
                        f: F
                    ) -> R
                    where
                        M: PinMode,
                        F: FnOnce(&mut $PXi<M>) -> R,
                    {
                        struct ResetMode<'a, ORIG: PinMode> {
                            pin: &'a mut $PXi<ORIG>,
                        }

                        impl<'a, ORIG: PinMode> Drop for ResetMode<'a, ORIG> {
                            fn drop(&mut self) {
                                self.pin.mode::<ORIG>();
                            }
                        }

                        self.mode::<M>();

                        // This will reset the pin back to the original mode when dropped.
                        // (so either when `with_mode` returns or when `f` unwinds)
                        let _resetti = ResetMode { pin: self };

                        let mut witness = $PXi {
                            _mode: PhantomData
                        };

                        f(&mut witness)
                    }

                    /// Configures the pin to operate as a floating input pin.
                    pub fn into_floating_input(
                        mut self,
                    ) -> $PXi<Input<Floating>> {
                        self.mode::<Input<Floating>>();
                        $PXi {
                            _mode: PhantomData
                        }
                    }

                    /// Temporarily configures this pin as a floating input.
                    ///
                    /// The closure `f` is called with the reconfigured pin. After it returns,
                    /// the pin will be configured back.
                    pub fn with_floating_input<R>(
                        &mut self,
                        f: impl FnOnce(&mut $PXi<Input<Floating>>) -> R,
                    ) -> R {
                        self.with_mode(f)
                    }

                    /// Configures the pin to operate as a pulled-down input pin.
                    pub fn into_pull_down_input(
                        mut self,
                    ) -> $PXi<Input<PullDown>> {
                        self.mode::<Input<PullDown>>();
                        $PXi {
                            _mode: PhantomData
                        }
                    }

                    /// Temporarily configures this pin as a pulled-down input.
                    ///
                    /// The closure `f` is called with the reconfigured pin. After it returns,
                    /// the pin will be configured back.
                    pub fn with_pull_down_input<R>(
                        &mut self,
                        f: impl FnOnce(&mut $PXi<Input<PullDown>>) -> R,
                    ) -> R {
                        self.with_mode(f)
                    }

                    /// Configures the pin to operate as a pulled-up input pin.
                    pub fn into_pull_up_input(
                        mut self,
                    ) -> $PXi<Input<PullUp>> {
                        self.mode::<Input<PullUp>>();
                        $PXi {
                            _mode: PhantomData
                        }
                    }

                    /// Temporarily configures this pin as a pulled-up input.
                    ///
                    /// The closure `f` is called with the reconfigured pin. After it returns,
                    /// the pin will be configured back.
                    pub fn with_pull_up_input<R>(
                        &mut self,
                        f: impl FnOnce(&mut $PXi<Input<PullUp>>) -> R,
                    ) -> R {
                        self.with_mode(f)
                    }

                    /// Configures the pin to operate as an analog pin.
                    pub fn into_analog(
                        mut self,
                    ) -> $PXi<Analog> {
                        self.mode::<Analog>();
                        $PXi {
                            _mode: PhantomData
                        }
                    }

                    /// Temporarily configures this pin as an analog pin.
                    ///
                    /// The closure `f` is called with the reconfigured pin. After it returns,
                    /// the pin will be configured back.
                    pub fn with_analog<R>(
                        &mut self,
                        f: impl FnOnce(&mut $PXi<Analog>) -> R,
                    ) -> R {
                        self.with_mode(f)
                    }

                    /// Configures the pin to operate as an open drain output pin.
                    pub fn into_open_drain_output(
                        mut self,
                    ) -> $PXi<Output<OpenDrain>> {
                        self.mode::<Output<OpenDrain>>();
                        $PXi {
                            _mode: PhantomData
                        }
                    }

                    /// Temporarily configures this pin as an open drain output.
                    ///
                    /// The closure `f` is called with the reconfigured pin. After it returns,
                    /// the pin will be configured back.
                    pub fn with_open_drain_output<R>(
                        &mut self,
                        f: impl FnOnce(&mut $PXi<Output<OpenDrain>>) -> R,
                    ) -> R {
                        self.with_mode(f)
                    }

                    /// Configures the pin to operate as an push-pull output pin.
                    pub fn into_push_pull_output(
                        mut self,
                    ) -> $PXi<Output<PushPull>> {
                        self.mode::<Output<PushPull>>();
                        $PXi {
                            _mode: PhantomData
                        }
                    }

                    /// Temporarily configures this pin as a push-pull output.
                    ///
                    /// The closure `f` is called with the reconfigured pin. After it returns,
                    /// the pin will be configured back.
                    pub fn with_push_pull_output<R>(
                        &mut self,
                        f: impl FnOnce(&mut $PXi<Output<PushPull>>) -> R,
                    ) -> R {
                        self.with_mode(f)
                    }

                    /// Set pin speed.
                    pub fn set_speed(self, speed: Speed) -> Self {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).ospeedr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | ((speed as u32) << offset))
                            })
                        };
                        self
                    }

                    #[allow(dead_code)]
                    pub(crate) fn set_alt_mode(&self, mode: AltMode) {
                        let mode = mode as u32;
                        let offset = 2 * $i;
                        let offset2 = 4 * $i;
                        unsafe {
                            if offset2 < 32 {
                                (*$GPIOX::ptr()).afrl.modify(|r, w| {
                                    w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
                                });
                            } else {
                                let offset2 = offset2 - 32;
                                (*$GPIOX::ptr()).afrh.modify(|r, w| {
                                    w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
                                });
                            }
                            (*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b10 << offset))
                            });
                        }
                    }
                }

                impl<MODE> $PXi<Output<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> Pin<Output<MODE>> {
                        Pin {
                            i: $i,
                            port: $GPIOX::ptr() as *const dyn GpioRegExt,
                            _mode: self._mode,
                        }
                    }
                }

                impl<MODE> OutputPin for $PXi<Output<MODE>> {
                    type Error = void::Void;

                    fn set_high(&mut self) -> Result<(), Self::Error> {
                        // NOTE(unsafe) atomic write to a stateless register
                        unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << $i)) };
                        Ok(())
                    }

                    fn set_low(&mut self) -> Result<(), Self::Error> {
                        // NOTE(unsafe) atomic write to a stateless register
                        unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << ($i + 16))) };
                        Ok(())
                    }
                }

                impl<MODE> StatefulOutputPin for $PXi<Output<MODE>> {

                    fn is_set_high(&self) -> Result<bool, Self::Error> {
                        let is_set_high = !self.is_set_low()?;
                        Ok(is_set_high)
                    }

                    fn is_set_low(&self) -> Result<bool, Self::Error> {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_set_low = unsafe { (*$GPIOX::ptr()).odr.read().bits() & (1 << $i) == 0 };
                        Ok(is_set_low)
                    }
                }

                impl<MODE> toggleable::Default for $PXi<Output<MODE>> {}

                impl<MODE> InputPin for $PXi<Output<MODE>> {
                    type Error = void::Void;

                    fn is_high(&self) -> Result<bool, Self::Error> {
                        let is_high = !self.is_low()?;
                        Ok(is_high)
                    }

                    fn is_low(&self) -> Result<bool, Self::Error> {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_low = unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << $i) == 0 };
                        Ok(is_low)
                    }
                }

                impl<MODE> $PXi<Input<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> Pin<Input<MODE>> {
                        Pin {
                            i: $i,
                            port: $GPIOX::ptr() as *const dyn GpioRegExt,
                            _mode: self._mode,
                        }
                    }
                }

                impl<MODE> InputPin for $PXi<Input<MODE>> {
                    type Error = void::Void;

                    fn is_high(&self) -> Result<bool, Self::Error> {
                        let is_high = !self.is_low()?;
                        Ok(is_high)
                    }

                    fn is_low(&self) -> Result<bool, Self::Error> {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_low = unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << $i) == 0 };
                        Ok(is_low)
                    }
                }
            )+
        }
    }
}

gpio!(GPIOA, gpioa, PA, [
    PA0: (pa0, 0, Analog),
    PA1: (pa1, 1, Analog),
    PA2: (pa2, 2, Analog),
    PA3: (pa3, 3, Analog),
    PA4: (pa4, 4, Analog),
    PA5: (pa5, 5, Analog),
    PA6: (pa6, 6, Analog),
    PA7: (pa7, 7, Analog),
    PA8: (pa8, 8, Analog),
    PA9: (pa9, 9, Analog),
    PA10: (pa10, 10, Analog),
    PA11: (pa11, 11, Analog),
    PA12: (pa12, 12, Analog),
    PA13: (pa13, 13, Analog),
    PA14: (pa14, 14, Analog),
    PA15: (pa15, 15, Analog),
]);

gpio!(GPIOB, gpiob, PB, [
    PB0: (pb0, 0, Analog),
    PB1: (pb1, 1, Analog),
    PB2: (pb2, 2, Analog),
    PB3: (pb3, 3, Analog),
    PB4: (pb4, 4, Analog),
    PB5: (pb5, 5, Analog),
    PB6: (pb6, 6, Analog),
    PB7: (pb7, 7, Analog),
    PB8: (pb8, 8, Analog),
    PB9: (pb9, 9, Analog),
    PB10: (pb10, 10, Analog),
    PB11: (pb11, 11, Analog),
    PB12: (pb12, 12, Analog),
    PB13: (pb13, 13, Analog),
    PB14: (pb14, 14, Analog),
    PB15: (pb15, 15, Analog),
]);

gpio!(GPIOC, gpioc, PC, [
    PC0: (pc0, 0, Analog),
    PC1: (pc1, 1, Analog),
    PC2: (pc2, 2, Analog),
    PC3: (pc3, 3, Analog),
    PC4: (pc4, 4, Analog),
    PC5: (pc5, 5, Analog),
    PC6: (pc6, 6, Analog),
    PC7: (pc7, 7, Analog),
    PC8: (pc8, 8, Analog),
    PC9: (pc9, 9, Analog),
    PC10: (pc10, 10, Analog),
    PC11: (pc11, 11, Analog),
    PC12: (pc12, 12, Analog),
    PC13: (pc13, 13, Analog),
    PC14: (pc14, 14, Analog),
    PC15: (pc15, 15, Analog),
]);

gpio!(GPIOD, gpiod, PD, [
    PD0: (pd0, 0, Analog),
    PD1: (pd1, 1, Analog),
    PD2: (pd2, 2, Analog),
    PD3: (pd3, 3, Analog),
    PD4: (pd4, 4, Analog),
    PD5: (pd5, 5, Analog),
    PD6: (pd6, 6, Analog),
    PD7: (pd7, 7, Analog),
    PD8: (pd8, 8, Analog),
    PD9: (pd9, 9, Analog),
    PD10: (pd10, 10, Analog),
    PD11: (pd11, 11, Analog),
    PD12: (pd12, 12, Analog),
    PD13: (pd13, 13, Analog),
    PD14: (pd14, 14, Analog),
    PD15: (pd15, 15, Analog),
]);

gpio!(GPIOE, gpioe, PE, [
    PE0:  (pe0,  0,  Analog),
    PE1:  (pe1,  1,  Analog),
    PE2:  (pe2,  2,  Analog),
    PE3:  (pe3,  3,  Analog),
    PE4:  (pe4,  4,  Analog),
    PE5:  (pe5,  5,  Analog),
    PE6:  (pe6,  6,  Analog),
    PE7:  (pe7,  7,  Analog),
    PE8:  (pe8,  8,  Analog),
    PE9:  (pe9,  9,  Analog),
    PE10: (pe10, 10, Analog),
    PE11: (pe11, 11, Analog),
    PE12: (pe12, 12, Analog),
    PE13: (pe13, 13, Analog),
    PE14: (pe14, 14, Analog),
    PE15: (pe15, 15, Analog),
]);

gpio!(GPIOH, gpioh, PH, [
    PH0: (ph0, 0, Analog),
    PH1: (ph1, 1, Analog),
    PH9: (ph9, 9, Analog),
    PH10: (ph10, 10, Analog),
]);
