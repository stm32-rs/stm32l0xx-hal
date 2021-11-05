use crate::rcc::{Enable, Rcc, Reset, HSI48};

pub use crate::pac::{rng, RNG};

pub struct Rng {
    rng: RNG,
}

impl Rng {
    // Initializes the peripheral
    pub fn new(rng: RNG, rcc: &mut Rcc, _: HSI48) -> Rng {
        // Enable peripheral clock
        RNG::enable(rcc);
        // Reset peripheral
        RNG::reset(rcc);

        rng.cr.write(|w| w.rngen().set_bit().ie().clear_bit());

        let mut ret = Self { rng };

        ret.enable();

        ret
    }

    pub fn enable(&mut self) {
        self.rng.cr.write(|w| w.rngen().set_bit().ie().clear_bit());
    }

    pub fn disable(&mut self) {
        self.rng
            .cr
            .modify(|_, w| w.rngen().clear_bit().ie().clear_bit());
    }

    pub fn wait(&mut self) {
        while self.rng.sr.read().drdy().bit_is_clear() {}
    }

    pub fn take_result(&mut self) -> u32 {
        self.rng.dr.read().bits()
    }
}
