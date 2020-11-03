//! Interface to the FLASH peripheral
//!
//! See STM32L0x2 reference manual, chapter 3.

use cortex_m::interrupt;

use crate::{
    pac::{self, flash::acr::LATENCY_A},
    rcc::Rcc,
};

/// The first address of flash memory
pub const FLASH_START: u32 = 0x0800_0000;

/// The size of a Flash memory page, in bytes
pub const PAGE_SIZE: u32 = 128;

/// Entry point to the non-volatile memory (NVM) API
pub struct FLASH {
    flash: pac::FLASH,
    flash_end: u32,
    eeprom_start: u32,
    eeprom_end: u32,
}

impl FLASH {
    // Initializes the FLASH peripheral
    pub fn new(flash: pac::FLASH, rcc: &mut Rcc) -> Self {
        // Determine size of the flash memory. According to the STM32L0x2
        // reference manual, section 33.1, there's a register that we can get
        // that information from. It doesn't seem to be exposed through the PAC,
        // so we have to read it manually.
        //
        // This is safe, as we're reading from a valid address (as per the
        // reference manual) which is aligned to 16 bits.
        let flash_size_in_kb = flash_size_in_kb();
        let flash_end = FLASH_START + flash_size_in_kb * 1024;

        // As of this writing, this module is only enabled for STM32L0x2.
        // According to the STM32L0x2 reference manual, section 1.4, the
        // following should determine whether this is a Category 5 device.
        // Please make sure to adapt this when porting this module to other
        // targets.
        let is_category_5 = cfg!(feature = "io-STM32L071");

        // Determine the start of the EEPROM, according to the tables in the
        // STM32L0x2 reference manual, section 3.3.1.
        let eeprom_start = if is_category_5 && flash_size_in_kb == 64 {
            // See table 10.
            0x0808_0C00
        } else {
            0x0808_0000
        };

        // Determine the end of the EEPROM. Please note that the tables in
        // section 3.3.1 specify the last byte of the EEPROM, while this is the
        // first byte after it.
        let eeprom_end = if is_category_5 {
            0x0808_1800
        } else {
            0x0808_0800
        };

        // Reset the peripheral interface
        rcc.rb.ahbrstr.modify(|_, w| w.mifrst().set_bit());
        rcc.rb.ahbrstr.modify(|_, w| w.mifrst().clear_bit());

        // Enable the peripheral interface
        rcc.rb.ahbenr.modify(|_, w| w.mifen().set_bit());

        Self {
            flash,
            flash_end,
            eeprom_start,
            eeprom_end,
        }
    }

    /// Set wait states
    ///
    /// By default, the number of wait states is zero. This is not suitable for
    /// all configurations. Depending on the processor's voltage range and
    /// frequency, it might be necessary to set the number of wait states to 1.
    ///
    /// This is explained, for example, in the STM32L0x2 Reference Manual,
    /// section 3.3.3.
    pub fn set_wait_states(&mut self, wait_states: LATENCY_A) {
        self.flash
            .acr
            .modify(|_, w| w.latency().variant(wait_states));
    }

    /// Erases a page of flash memory
    ///
    /// Attention: You must make sure that your program is not executed from the
    /// same Flash bank that the page is being erased in. Either make sure your
    /// program is executed from another Flash bank, or run it from main memory.
    ///
    /// # Panics
    ///
    /// This method will panic, unless all of the following is true:
    /// - `address` points to Flash memory
    /// - `address` is aligned to a page boundary (32 words, 128 bytes)
    pub fn erase_flash_page(&mut self, address: *mut u32) -> Result {
        self.unlock(|self_| {
            let memory = self_.verify_address(address);

            if !memory.is_flash() {
                panic!("Address does not point to Flash memory");
            }
            if address as u32 & 0x7f != 0 {
                panic!("Address is not aligned to page boundary");
            }

            // Wait, while the memory interface is busy.
            while self_.flash.sr.read().bsy().is_active() {}

            // Enable erase operation
            self_.flash.pecr.modify(|_, w| {
                // Required to erase
                w.erase().set_bit();
                // Required for mass operations in Flash memory
                w.prog().set_bit();

                w
            });

            // Erase page
            // Safe, as we know that this points to Flash memory.
            unsafe { address.write_volatile(0) }

            // Wait for operation to complete
            while self_.flash.sr.read().bsy().is_active() {}

            self_.check_errors()

            // No need to reset PECR flags, that's done by `unlock`.
        })
    }

    /// Writes a word to Flash memory or EEPROM
    ///
    /// Please note that any access to Flash or EEPROM on the same memory bank
    /// will be stalled until this operation completes.
    ///
    /// If you use this method to write to Flash memory, the address must have
    /// been erased before, otherwise this method will return an error.
    ///
    /// # Panics
    ///
    /// Panics, if `address` does not point to Flash memory or EEPROM.
    pub fn write_word(&mut self, address: *mut u32, word: u32) -> Result {
        self.unlock(|self_| {
            self_.verify_address(address);

            // Wait, while the memory interface is busy.
            while self_.flash.sr.read().bsy().is_active() {}

            // Write memory
            // Safe, as we know that this points to flash or EEPROM.
            unsafe { address.write_volatile(word) }

            // Wait for operation to complete
            while self_.flash.sr.read().bsy().is_active() {}

            self_.check_errors()
        })
    }

    /// Writes a half-page (16 words) of Flash  memory
    ///
    /// The memory written to must have been erased before, otherwise this
    /// method will return an error.
    ///
    /// # Panics
    ///
    /// This method will panic, unless all of the following is true:
    /// - `address` points to Flash memory
    /// - `address` is aligned to a half-page boundary (16 words, 64 bytes)
    /// - `words` has a length of 16
    pub fn write_flash_half_page(&mut self, address: *mut u32, words: &[u32]) -> Result {
        self.unlock(|self_| {
            let memory = self_.verify_address(address);

            if !memory.is_flash() {
                panic!("Address does not point to Flash memory");
            }
            if address as u32 & 0x3f != 0 {
                panic!("Address is not aligned to half-page boundary");
            }
            if words.len() != 16 {
                panic!("`words` is not exactly a half-page of memory");
            }

            // Wait, while the memory interface is busy.
            while self_.flash.sr.read().bsy().is_active() {}

            // Enable write operation
            self_.flash.pecr.modify(|_, w| {
                // Half-page programming mode
                w.fprg().set_bit();
                // Required for mass operations in Flash memory
                w.prog().set_bit();

                w
            });

            // We absoluty can't have any access to Flash while preparing the
            // write, or the process will be interrupted. This includes any
            // access to the vector table or interrupt handlers that might be
            // caused by an interrupt.
            interrupt::free(|_| {
                // Safe, because we've verified the valididty of `address` and
                // the length of `words`.
                unsafe {
                    write_half_page(address, words.as_ptr());
                }
            });

            // Wait for operation to complete
            while self_.flash.sr.read().bsy().is_active() {}

            self_.check_errors()

            // No need to reset PECR flags, that's done by `unlock`.
        })
    }

    fn unlock(&mut self, f: impl FnOnce(&mut Self) -> Result) -> Result {
        // Unlock everything that needs unlocking
        self.flash.pekeyr.write(|w| w.pekeyr().bits(0x89ABCDEF));
        self.flash.pekeyr.write(|w| w.pekeyr().bits(0x02030405));
        self.flash.prgkeyr.write(|w| w.prgkeyr().bits(0x8C9DAEBF));
        self.flash.prgkeyr.write(|w| w.prgkeyr().bits(0x13141516));
        self.flash.optkeyr.write(|w| w.optkeyr().bits(0xFBEAD9C8));
        self.flash.optkeyr.write(|w| w.optkeyr().bits(0x24252627));

        let result = f(self);

        // Reset operations and write protection
        self.flash.pecr.reset();

        result
    }

    fn verify_address(&self, address: *mut u32) -> Memory {
        let address = address as u32;

        let memory = match address {
            _ if FLASH_START <= address && address < self.flash_end => Memory::Flash,
            _ if self.eeprom_start <= address && address < self.eeprom_end => Memory::Eeprom,
            _ => Memory::Other,
        };

        if memory.is_other() {
            panic!("Address is neither in Flash memory nor EEPROM");
        }

        memory
    }

    pub fn check_errors(&self) -> Result {
        let sr = self.flash.sr.read();

        if sr.fwwerr().bit_is_set() {
            // Reset flag
            self.flash.sr.write(|w| w.fwwerr().set_bit());

            return Err(Error::AbortedByFetch);
        }
        if sr.notzeroerr().bit_is_set() {
            // Reset flag
            self.flash.sr.write(|w| w.notzeroerr().set_bit());

            return Err(Error::NotErased);
        }
        if sr.rderr().bit_is_set() {
            // Reset flag
            self.flash.sr.write(|w| w.rderr().set_bit());

            return Err(Error::ReadProtection);
        }
        if sr.optverr().bit_is_set() {
            // Reset flag
            self.flash.sr.write(|w| w.optverr().set_bit());

            return Err(Error::ConfigMismatch);
        }
        if sr.sizerr().bit_is_set() {
            // Reset flag
            self.flash.sr.write(|w| w.sizerr().set_bit());

            return Err(Error::InvalidSize);
        }
        if sr.pgaerr().bit_is_set() {
            // Reset flag
            self.flash.sr.write(|w| w.pgaerr().set_bit());

            return Err(Error::InvalidAlignment);
        }
        if sr.wrperr().bit_is_set() {
            // Reset flag
            self.flash.sr.write(|w| w.wrperr().set_bit());

            return Err(Error::WriteProtection);
        }

        Ok(())
    }
}

pub fn flash_size_in_kb() -> u32 {
    // Determine size of the flash memory. According to the STM32L0x2 reference
    // manual, section 33.1, there's a register that we can get that information
    // from. It doesn't seem to be exposed through the PAC, so we have to read
    // it manually.
    //
    // This is safe, as we're reading from a valid address (as per the
    // reference manual) which is aligned to 16 bits.
    unsafe { (0x1FF8_007C as *const u16).read() as u32 }
}

extern "C" {
    /// Writes a half-page at the given address
    ///
    /// Unfortunately this function had to be implemented in C. No access to
    /// Flash memory is allowed after the first word has been written, and that
    /// includes execution of code that is located in Flash. This means the
    /// function that writes the half-page has to be executed from memory, and
    /// is not allowed to call any functions that are not located in memory.
    ///
    /// Unfortunately I found this impossible to achieve in Rust. I can write
    /// a Rust function that is located in RAM, using `#[link_section=".data"]`,
    /// but I failed to write any useful Rust code that doesn't include function
    /// calls to _something_ that is outside of my control, as so much of Rust's
    /// functionality is defined in terms of function calls.
    ///
    /// I ended up writing it in C, as that was the only solution I could come
    /// up with that will run on the stable channel (is nightly is acceptable,
    /// we could use a Rust function with inline assembly).
    fn write_half_page(address: *mut u32, words: *const u32);
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Memory {
    Flash,
    Eeprom,
    Other,
}

impl Memory {
    fn is_flash(&self) -> bool {
        *self == Memory::Flash
    }

    // The following method is not currently used, but I left it here, in case
    // new methods need this functionality later.
    // fn is_eeprom(&self) -> bool {
    //     *self == Memory::Eeprom
    // }

    fn is_other(&self) -> bool {
        *self == Memory::Other
    }
}

type Result = core::result::Result<(), Error>;

#[derive(Debug)]
pub enum Error {
    /// Write/erase was aborted by fetch operation
    ///
    /// See FWWERR bit in SR register.
    AbortedByFetch,

    /// Failed to write memory that was not erased
    ///
    /// See NOTZEROERR bit in SR register.
    NotErased,

    /// Attempted to read protected memory
    ///
    /// See RDERR bit in SR register.
    ReadProtection,

    /// Configuration mismatch
    ///
    /// See OPTVERR bit in SR register.
    ConfigMismatch,

    /// Size of data to program is not correct
    ///
    /// See SIZERR bit in SR register.
    InvalidSize,

    /// Incorrect alignment when programming half-page
    ///
    /// See PGAERR bit in SR register.
    InvalidAlignment,

    /// Attempted to write to protected memory
    ///
    /// See WRPERR in SR register.
    WriteProtection,
}
