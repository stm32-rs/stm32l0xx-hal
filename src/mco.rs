//! MCO (Microcontroller Clock Output)
//!
//! MCO is available on PA8 or PA9. See "Table 16. Alternate function port A" in the datasheet.

use crate::gpio::{gpioa, gpiob, AltMode, Analog};

pub trait Pin {
    fn into_mco(self);
}

impl Pin for gpioa::PA8<Analog> {
    fn into_mco(self) {
        self.set_alt_mode(AltMode::AF0);
    }
}

impl Pin for gpioa::PA9<Analog> {
    fn into_mco(self) {
        self.set_alt_mode(AltMode::AF0);
    }
}

impl Pin for gpiob::PB13<Analog> {
    fn into_mco(self) {
        self.set_alt_mode(AltMode::AF2);
    }
}

// Blanket impls to allow configuration of all MCO pins.
impl<P1, P2> Pin for (P1, P2)
where
    P1: Pin,
    P2: Pin,
{
    fn into_mco(self) {
        self.0.into_mco();
        self.1.into_mco();
    }
}

impl<P1, P2, P3> Pin for (P1, P2, P3)
where
    P1: Pin,
    P2: Pin,
    P3: Pin,
{
    fn into_mco(self) {
        self.0.into_mco();
        self.1.into_mco();
        self.2.into_mco();
    }
}
