//! Interface to the FLASH peripheral
//!
//! This manages access to both the program flash memory, as well as EEPROM.
//!
//! References:
//!
//! - STM32L0x1 reference manual (RM0377), chapter 3
//! - STM32L0x2 reference manual (RM0376), chapter 3
//! - STM32L0x3 reference manual (RM0367), chapter 3

use cortex_m::interrupt;

use crate::{
    pac::{self, flash::acr::LATENCY_A},
    rcc::{Enable, Rcc, Reset},
};

/// The first address of flash memory
pub const FLASH_START: usize = 0x0800_0000;

/// The size of a Flash memory page, in bytes
pub const PAGE_SIZE: usize = 128;

// EEPROM sizes in bytes, generated with cube-parse
#[cfg(feature = "eeprom-256")]
pub const EEPROM_SIZE: usize = 256;
#[cfg(feature = "eeprom-1024")]
pub const EEPROM_SIZE: usize = 1024;
#[cfg(feature = "eeprom-3072")]
pub const EEPROM_SIZE: usize = 3072;
#[cfg(feature = "eeprom-2048")]
pub const EEPROM_SIZE: usize = 2048;
#[cfg(feature = "eeprom-6144")]
pub const EEPROM_SIZE: usize = 6144;
#[cfg(feature = "eeprom-512")]
pub const EEPROM_SIZE: usize = 512;
#[cfg(feature = "eeprom-128")]
pub const EEPROM_SIZE: usize = 128;

// EEPROM start addresses
pub const EEPROM_START_BANK1: usize = 0x0808_0000;
pub const EEPROM_START_BANK2: usize = 0x0808_0C00;

/// Entry point to the non-volatile memory (NVM) API
pub struct FLASH {
    flash: pac::FLASH,
    flash_end: usize,
    eeprom_start: usize,
    eeprom_end: usize,
}

impl FLASH {
    /// Initializes the FLASH peripheral
    pub fn new(flash: pac::FLASH, rcc: &mut Rcc) -> Self {
        // Determine size of the flash memory
        let flash_size_in_kb = flash_size_in_kb();
        let flash_end = FLASH_START + flash_size_in_kb * 1024;

        // Determine the start of the EEPROM. Most MCUs have two EEPROM banks,
        // but some have only one bank (BANK2), see for example STM32L0x2
        // reference manual, table 10. At the time of this writing, this can be
        // detected by checking both the flash and EEPROM size.
        //
        // Note: In contrast to flash size, EEPROM size cannot be determined at
        // runtime, so we rely on the proper `EEPROM_SIZE` const being
        // set.
        let eeprom_start = if flash_size_in_kb == 64 && EEPROM_SIZE == 3072 {
            EEPROM_START_BANK2
        } else {
            EEPROM_START_BANK1
        };

        // Determine the end of the EEPROM. Please note that the tables in
        // section 3.3.1 specify the last byte of the EEPROM, while this is the
        // first byte after it.
        let eeprom_end = eeprom_start + EEPROM_SIZE;

        // Enable the peripheral interface
        pac::FLASH::enable(rcc);
        // Reset the peripheral interface
        pac::FLASH::reset(rcc);

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

    /// Writes a single byte to EEPROM
    ///
    /// Please note that any access to Flash or EEPROM on the same memory bank
    /// will be stalled until this operation completes.
    ///
    /// # Constant Time Writes
    ///
    /// Note that the write operation does not complete in constant time. If
    /// all bits of the current value in EEPROM are set to 0, the new value is
    /// written directly in `Tprog` (3.2 ms on the STM32L0x1). Otherwise, an
    /// erase operation is executed first, resulting in a total duration of
    /// 2x`Tprog` (6.4 ms on the STM32L0x1).
    ///
    /// If constant time writes are important, you could set the FIX bit to
    /// force the memory interface to always execute an erase before writing
    /// new data. However, this is not currently supported in the HAL.
    ///
    /// # Panics
    ///
    /// Panics, if `address` does not point to EEPROM.
    pub fn write_byte(&mut self, address: *mut u8, byte: u8) -> Result {
        self.unlock(|self_| {
            // Verify that the address points to EEPROM
            let memory = self_.verify_address(address);
            if !memory.is_eeprom() {
                panic!("Address does not point to EEPROM memory");
            }

            // Wait, while the memory interface is busy.
            while self_.flash.sr.read().bsy().is_active() {}

            // Write memory
            // Safe, as we know that this points to flash or EEPROM.
            unsafe { address.write_volatile(byte) }

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
            if address as usize & 0x3f != 0 {
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

            // No need to manually reset PECR flags, that's done by `unlock`.
        })
    }

    /// Unlock everything that needs unlocking:
    ///
    /// - FLASH_PECR lock (PELOCK)
    /// - Program memory lock (PRGLOCK)
    /// - Option bytes lock (OPTLOCK)
    ///
    /// Then, once unlocked, run the provided function.
    ///
    /// References:
    ///
    /// - STM32L0x1 reference manual (RM0377), section 3.3.4 (Writing/erasing the NVM)
    fn unlock(&mut self, f: impl FnOnce(&mut Self) -> Result) -> Result {
        // FLASH_PECR lock
        self.flash.pekeyr.write(|w| w.pekeyr().bits(0x89ABCDEF));
        self.flash.pekeyr.write(|w| w.pekeyr().bits(0x02030405));
        // Program memory lock
        self.flash.prgkeyr.write(|w| w.prgkeyr().bits(0x8C9DAEBF));
        self.flash.prgkeyr.write(|w| w.prgkeyr().bits(0x13141516));
        // Option bytes lock
        self.flash.optkeyr.write(|w| w.optkeyr().bits(0xFBEAD9C8));
        self.flash.optkeyr.write(|w| w.optkeyr().bits(0x24252627));

        let result = f(self);

        // Reset operations and write protection
        self.flash.pecr.reset();

        result
    }

    fn verify_address<T>(&self, address: *mut T) -> Memory {
        let address = address as usize;

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

    /// Check for errors.
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

// Determine size of the flash memory in KiB.
//
// This information can be read from the "Flash size register".
//
// Reference:
//
// - STM32L0x1 reference manual, section 28.1.1
// - STM32L0x2 reference manual, section 33.1.1
// - STM32L0x3 reference manual, section 34.1.1
pub fn flash_size_in_kb() -> usize {
    // This is safe, as we're reading from a valid address (as per the
    // reference manual) which is aligned to 16 bits.
    unsafe { (0x1FF8_007C as *const u16).read() as usize }
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

    fn is_eeprom(&self) -> bool {
        *self == Memory::Eeprom
    }

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
