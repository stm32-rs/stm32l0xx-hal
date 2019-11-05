use crate::rcc::{HSI48, Rcc};

pub use crate::pac::{rng, RNG};


pub struct Rng {
    rng: RNG
}

impl Rng {
    // Initializes the peripheral
    pub fn new(rng: RNG, rcc: &mut Rcc, _: HSI48) -> Rng {
        // Reset peripheral
        rcc.rb.ahbrstr.modify(|_, w| w.rngrst().set_bit());
        rcc.rb.ahbrstr.modify(|_, w| w.rngrst().clear_bit());

        // Enable peripheral clock
        rcc.rb.ahbenr.modify(|_, w| w.rngen().set_bit());

        rng.cr.write(|w| 
            w
                .rngen().set_bit()
                .ie().clear_bit()
        );

        let mut ret = Self {
            rng
        };

        ret.enable();

        ret
    }

    pub fn enable(&mut self) {
        self.rng.cr.write(|w| 
            w
                .rngen().set_bit()
                .ie().clear_bit()
        );
    }

    pub fn disable(&mut self) {
        self.rng.cr.modify(|_, w| 
            w
                .rngen().clear_bit()
                .ie().clear_bit()
        );
    }

    pub fn wait(&mut self) {
        while self.rng.sr.read().drdy().bit_is_clear() {}
    }

    pub fn take_result(&mut self) -> u32 {
        self.rng.dr.read().bits()
    }
}
