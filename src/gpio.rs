//! General Purpose Input / Output

use core::marker::PhantomData;

use crate::rcc::Rcc;

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
		/// The parts to split the GPIO into
		type Parts;

		/// Splits the GPIO block into independent pins and registers
		fn split(self, rcc: &mut Rcc) -> Self::Parts;
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

/// Alternative mode Open drain output (type state)
pub struct AltOpenDrain;

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

impl sealed::Sealed for Output<AltOpenDrain> {}
impl PinMode for Output<AltOpenDrain> {
    const PUPDR: u8 = 0b10;
    const MODER: u8 = 0b10010;
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

macro_rules! gpio {
		($GPIOX:ident, $gpiox:ident, $gpioy:ident, $iopxenr:ident, $iopxrst:ident, $PXx:ident, [
				$($PXi:ident: ($pxi:ident, $i:expr, $MODE:ty, $AFR:ident),)+
		]) => {
				/// GPIO
				pub mod $gpiox {
						use core::marker::PhantomData;

						use crate::hal::digital::v2::{toggleable, InputPin, OutputPin, StatefulOutputPin};
						use crate::pac::{$GPIOX, $gpioy};
						use crate::rcc::{Rcc};
						use super::{
								Floating, GpioExt, Input, OpenDrain, Output, Speed,
								PullDown, PullUp, PushPull, AltMode, Analog, Port,
								PinMode, AltOpenDrain,
						};

						/// GPIO parts
						pub struct Parts {
								/// Opaque AFRH register
								pub afrh: AFRH,
								/// Opaque AFRL register
								pub afrl: AFRL,
								/// Opaque MODER register
								pub moder: MODER,
								/// Opaque OTYPER register
								pub otyper: OTYPER,
								/// Opaque PUPDR register
								pub pupdr: PUPDR,
								$(
										/// Pin
										pub $pxi: $PXi<$MODE>,
								)+
						}
						impl GpioExt for $GPIOX {
								type Parts = Parts;

								fn split(self, rcc: &mut Rcc) -> Parts {
										rcc.iopenr.enr().modify(|_, w| w.$iopxenr().enabled());
										rcc.ioprstr.rstr().modify(|_, w| w.$iopxrst().set_bit());
										rcc.ioprstr.rstr().modify(|_, w| w.$iopxrst().clear_bit());

										Parts {
												afrh: AFRH { _0: () },
												afrl: AFRL { _0: () },
												moder: MODER { _0: () },
												otyper: OTYPER { _0: () },
												pupdr: PUPDR { _0: () },
												$(
														$pxi: $PXi { _mode: PhantomData },
												)+
										}
								}
						}

						/// Opaque AFRL register
						pub struct AFRL {
								_0: (),
						}

						impl AFRL {
								pub(crate) fn afr(&mut self) -> &$gpioy::AFRL {
										unsafe { &(*$GPIOX::ptr()).afrl }
								}
						}

						/// Opaque AFRH register
						pub struct AFRH {
								_0: (),
						}

						impl AFRH {
								pub(crate) fn afr(&mut self) -> &$gpioy::AFRH {
										unsafe { &(*$GPIOX::ptr()).afrh }
								}
						}

						/// Opaque MODER register
						pub struct MODER {
								_0: (),
						}

						impl MODER {
								pub(crate) fn moder(&mut self) -> &$gpioy::MODER {
										unsafe { &(*$GPIOX::ptr()).moder }
								}
						}

						/// Opaque OTYPER register
						pub struct OTYPER {
								_0: (),
						}

						impl OTYPER {
								pub(crate) fn otyper(&mut self) -> &$gpioy::OTYPER {
										unsafe { &(*$GPIOX::ptr()).otyper }
								}
						}

						/// Opaque PUPDR register
						pub struct PUPDR {
								_0: (),
						}

						impl PUPDR {
								pub(crate) fn pupdr(&mut self) -> &$gpioy::PUPDR {
										unsafe { &(*$GPIOX::ptr()).pupdr }
								}
						}


						/// Partially erased pin
						pub struct $PXx<MODE> {
								i: u8,
								_mode: PhantomData<MODE>,
						}

						impl<MODE> $PXx<MODE> {
								/// The port this pin is part of.
								pub const PORT: Port = Port::$PXx;

								/// Returns the port this pin is part of.
								pub fn port(&self) -> Port {
										Port::$PXx
								}

								/// Returns this pin's number inside its port.
								pub fn pin_number(&self) -> u8 {
										self.i
								}
						}

						impl<MODE> OutputPin for $PXx<Output<MODE>> {
								type Error = void::Void;

								fn set_high(&mut self) -> Result<(), Self::Error> {
										// NOTE(unsafe) atomic write to a stateless register
										unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << self.i)) };
										Ok(())
								}

								fn set_low(&mut self) -> Result<(), Self::Error> {
										// NOTE(unsafe) atomic write to a stateless register
										unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << (self.i + 16))) };
										Ok(())
								}
						}

						impl<MODE> StatefulOutputPin for $PXx<Output<MODE>> {
								fn is_set_high(&self) -> Result<bool, Self::Error> {
										let is_high = self.is_set_low()?;
										Ok(is_high)
								}

								fn is_set_low(&self) -> Result<bool, Self::Error> {
										// NOTE(unsafe) atomic read with no side effects
										let is_low = unsafe { (*$GPIOX::ptr()).odr.read().bits() & (1 << self.i) == 0 };
										Ok(is_low)
								}
						}

						impl<MODE> toggleable::Default for $PXx<Output<MODE>> {}

						impl<MODE> InputPin for $PXx<Output<MODE>> {
								type Error = void::Void;

								fn is_high(&self) -> Result<bool, Self::Error> {
										let is_high = !self.is_low()?;
										Ok(is_high)
								}

								fn is_low(&self) -> Result<bool, Self::Error> {
										// NOTE(unsafe) atomic read with no side effects
										let is_low = unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << self.i) == 0 };
										Ok(is_low)
								}
						}

						impl<MODE> InputPin for $PXx<Input<MODE>> {
								type Error = void::Void;

								fn is_high(&self) -> Result<bool, Self::Error> {
										let is_high = !self.is_low()?;
										Ok(is_high)
								}

								fn is_low(&self) -> Result<bool, Self::Error> {
										// NOTE(unsafe) atomic read with no side effects
										let is_low = unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << self.i) == 0 };
										Ok(is_low)
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
														&(*$GPIOX::ptr()).pupdr.modify(|r, w| {
																w.bits((r.bits() & !(0b11 << offset)) | (u32::from(M::PUPDR) << offset))
														});

														if let Some(otyper) = M::OTYPER {
																&(*$GPIOX::ptr()).otyper.modify(|r, w| {
																		w.bits(r.bits() & !(0b1 << $i) | (u32::from(otyper) << $i))
																});
														}

														&(*$GPIOX::ptr()).moder.modify(|r, w| {
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

                    /// Configures the pin to operate as an open drain output pin.
                    pub fn into_alt_open_drain_output(
                        mut self,
                        mode: AltMode,
                    ) -> $PXi<Output<AltOpenDrain>> {
                        self.set_alt_mode(mode);
                        self.mode::<Output<AltOpenDrain>>();
                        $PXi {
                            _mode: PhantomData
                        }
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
										pub fn set_alt_mode(&self, mode: AltMode) {
												let mode = mode as u32;
												let offset = 2 * $i;
												let offset2 = 4 * $i;
												unsafe {
														if offset2 < 32 {
																&(*$GPIOX::ptr()).afrl.modify(|r, w| {
																		w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
																});
														} else {
																let offset2 = offset2 - 32;
																&(*$GPIOX::ptr()).afrh.modify(|r, w| {
																		w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
																});
														}
														&(*$GPIOX::ptr()).moder.modify(|r, w| {
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
										pub fn downgrade(self) -> $PXx<Output<MODE>> {
												$PXx {
														i: $i,
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
										pub fn downgrade(self) -> $PXx<Input<MODE>> {
												$PXx {
														i: $i,
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

gpio!(GPIOA, gpioa, gpioa, iopaen, ioparst, PA, [
		PA0: (pa0, 0, Analog, AFRL),
		PA1: (pa1, 1, Analog, AFRL),
		PA2: (pa2, 2, Analog, AFRL),
		PA3: (pa3, 3, Analog, AFRL),
		PA4: (pa4, 4, Analog, AFRL),
		PA5: (pa5, 5, Analog, AFRL),
		PA6: (pa6, 6, Analog, AFRL),
		PA7: (pa7, 7, Analog, AFRL),
		PA8: (pa8, 8, Analog, AFRH),
		PA9: (pa9, 9, Analog, AFRH),
		PA10: (pa10, 10, Analog, AFRH),
		PA11: (pa11, 11, Analog, AFRH),
		PA12: (pa12, 12, Analog, AFRH),
		PA13: (pa13, 13, Analog, AFRH),
		PA14: (pa14, 14, Analog, AFRH),
		PA15: (pa15, 15, Analog, AFRH),
]);

gpio!(GPIOB, gpiob, gpiob, iopben, iopbrst, PB, [
		PB0: (pb0, 0, Analog, AFRL),
		PB1: (pb1, 1, Analog, AFRL),
		PB2: (pb2, 2, Analog, AFRL),
		PB3: (pb3, 3, Analog, AFRL),
		PB4: (pb4, 4, Analog, AFRL),
		PB5: (pb5, 5, Analog, AFRL),
		PB6: (pb6, 6, Analog, AFRL),
		PB7: (pb7, 7, Analog, AFRL),
		PB8: (pb8, 8, Analog, AFRH),
		PB9: (pb9, 9, Analog, AFRH),
		PB10: (pb10, 10, Analog, AFRH),
		PB11: (pb11, 11, Analog, AFRH),
		PB12: (pb12, 12, Analog, AFRH),
		PB13: (pb13, 13, Analog, AFRH),
		PB14: (pb14, 14, Analog, AFRH),
		PB15: (pb15, 15, Analog, AFRH),
]);

// gpio[c,d,e,h] module are derived from gpiob
gpio!(GPIOC, gpioc, gpiob, iopcen, iopcrst, PC, [
		PC0: (pc0, 0, Analog, AFRL),
		PC1: (pc1, 1, Analog, AFRL),
		PC2: (pc2, 2, Analog, AFRL),
		PC3: (pc3, 3, Analog, AFRL),
		PC4: (pc4, 4, Analog, AFRL),
		PC5: (pc5, 5, Analog, AFRL),
		PC6: (pc6, 6, Analog, AFRL),
		PC7: (pc7, 7, Analog, AFRL),
		PC8: (pc8, 8, Analog, AFRH),
		PC9: (pc9, 9, Analog, AFRH),
		PC10: (pc10, 10, Analog, AFRH),
		PC11: (pc11, 11, Analog, AFRH),
		PC12: (pc12, 12, Analog, AFRH),
		PC13: (pc13, 13, Analog, AFRH),
		PC14: (pc14, 14, Analog, AFRH),
		PC15: (pc15, 15, Analog, AFRH),
]);

gpio!(GPIOD, gpiod, gpiob, iopden, iopdrst, PD, [
		PD0: (pd0, 0, Analog, AFRL),
		PD1: (pd1, 1, Analog, AFRL),
		PD2: (pd2, 2, Analog, AFRL),
		PD3: (pd3, 3, Analog, AFRL),
		PD4: (pd4, 4, Analog, AFRL),
		PD5: (pd5, 5, Analog, AFRL),
		PD6: (pd6, 6, Analog, AFRL),
		PD7: (pd7, 7, Analog, AFRL),
		PD8: (pd8, 8, Analog, AFRH),
		PD9: (pd9, 9, Analog, AFRH),
		PD10: (pd10, 10, Analog, AFRH),
		PD11: (pd11, 11, Analog, AFRH),
		PD12: (pd12, 12, Analog, AFRH),
		PD13: (pd13, 13, Analog, AFRH),
		PD14: (pd14, 14, Analog, AFRH),
		PD15: (pd15, 15, Analog, AFRH),
]);

gpio!(GPIOE, gpioe, gpiob, iopeen, ioperst, PE, [
		PE0:	(pe0,  0,  Analog, AFRL),
		PE1:	(pe1,  1,  Analog, AFRL),
		PE2:	(pe2,  2,  Analog, AFRL),
		PE3:	(pe3,  3,  Analog, AFRL),
		PE4:	(pe4,  4,  Analog, AFRL),
		PE5:	(pe5,  5,  Analog, AFRL),
		PE6:	(pe6,  6,  Analog, AFRL),
		PE7:	(pe7,  7,  Analog, AFRL),
		PE8:	(pe8,  8,  Analog, AFRH),
		PE9:	(pe9,  9,  Analog, AFRH),
		PE10: (pe10, 10, Analog, AFRH),
		PE11: (pe11, 11, Analog, AFRH),
		PE12: (pe12, 12, Analog, AFRH),
		PE13: (pe13, 13, Analog, AFRH),
		PE14: (pe14, 14, Analog, AFRH),
		PE15: (pe15, 15, Analog, AFRH),
]);

gpio!(GPIOH, gpioh, gpiob, iophen, iophrst, PH, [
		PH0: (ph0, 0, Analog, AFRL),
		PH1: (ph1, 1, Analog, AFRL),
		PH9: (ph9, 9, Analog, AFRH),
		PH10: (ph10, 10, Analog, AFRH),
]);
