use crate::rcc::Rcc;
use crate::syscfg::SYSCFG;
use crate::pac;

pub use crate::pac::{rng, RNG};


pub struct Rng {
    rng: RNG
}

impl Rng {
    // initializes the peripheral after enabling 48 MHz clock
    pub fn new(rng: RNG, rcc: &mut Rcc, syscfg: &mut SYSCFG, crs: pac::CRS) -> Rng {

        // enable 48 MHz clock which feed RNG and USB
        rcc.enable48_mhz(syscfg, crs);

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
