//! Interface to the AES peripheral
//!
//! See STM32L0x2 reference manual, chapter 18.


use core::convert::TryInto;

use crate::{
    pac,
    rcc::Rcc,
};


/// Entry point to the AES API
pub struct AES {
    aes: pac::AES,
}

impl AES {
    /// Initialize the AES peripheral
    pub fn new(aes: pac::AES, rcc: &mut Rcc) -> Self {
        // Reset peripheral
        rcc.rb.ahbrstr.modify(|_, w| w.cryprst().set_bit());
        rcc.rb.ahbrstr.modify(|_, w| w.cryprst().clear_bit());

        // Enable peripheral clock
        rcc.rb.ahbenr.modify(|_, w| w.crypen().set_bit());

        // Configure peripheral
        aes.cr.write(|w| {
            w
                // Disable DMA
                .dmaouten().clear_bit()
                .dmainen().clear_bit()
                // Disable interrupts
                .errie().clear_bit()
                .ccfie().clear_bit()
        });

        Self {
            aes,
        }
    }

    /// Start a CTR stream
    ///
    /// Will consume this AES instance and return another instance which is
    /// switched to CTR mode. While in CTR mode, you can use other methods to
    /// encrypt/decrypt data.
    pub fn start_ctr_stream(self, key: [u32; 4], init_vector: [u32; 3])
        -> CtrStream
    {
        // Initialize key
        self.aes.keyr0.write(|w| unsafe { w.bits(key[0]) });
        self.aes.keyr1.write(|w| unsafe { w.bits(key[1]) });
        self.aes.keyr2.write(|w| unsafe { w.bits(key[2]) });
        self.aes.keyr3.write(|w| unsafe { w.bits(key[3]) });

        // Initialize initialization vector
        //
        // See STM32L0x2 reference manual, table 78 on page 408.
        self.aes.ivr3.write(|w| unsafe { w.bits(init_vector[0]) });
        self.aes.ivr2.write(|w| unsafe { w.bits(init_vector[1]) });
        self.aes.ivr1.write(|w| unsafe { w.bits(init_vector[2]) });
        self.aes.ivr0.write(|w| unsafe { w.bits(0x0001) }); // counter

        self.aes.cr.modify(|_, w| {
            let w = unsafe {
                w
                    // Select Counter Mode (CTR) mode
                    .chmod().bits(0b10)
                    // These bits mean encryption mode, but in CTR mode,
                    // encryption and descryption are technically identical, so
                    // this is fine for either mode.
                    .mode().bits(0b00)
                    // Configure for stream of bytes
                    .datatype().bits(0b10)
            };
            // Enable peripheral
            w.en().set_bit()
        });

        CtrStream {
            aes: self,
        }
    }
}


pub struct CtrStream {
    aes: AES,
}

impl CtrStream {
    /// Processes one block of data
    ///
    /// In CTR mode, encrypting and decrypting work the same. If you pass a
    /// block of clear data to this function, an encrypted block is returned. If
    /// you pass a block of encrypted data, it is decrypted and a clear block
    /// is returned.
    pub fn process(&mut self, input: &Block) -> Block {
        // Write input data to DINR
        //
        // See STM32L0x2 reference manual, section 18.4.10.
        for i in (0 .. 4).rev() {
            self.aes.aes.dinr.write(|w| {
                let i = i * 4;

                let word = &input[i .. i+4];
                // Can't panic, because `word` is 4 bytes long.
                let word = word.try_into().unwrap();
                let word = u32::from_le_bytes(word);

                unsafe { w.bits(word) }
            });
        }

        // Wait while computation is not complete
        while self.aes.aes.sr.read().ccf().bit_is_clear() {}

        // Read output data from DOUTR
        //
        // See STM32L0x2 reference manual, section 18.4.10.
        let mut output = [0; 16];
        for i in (0 .. 4).rev() {
            let i = i * 4;

            let word = self.aes.aes.doutr.read().bits();
            let word = word.to_le_bytes();

            (&mut output[i .. i+4]).copy_from_slice(&word);
        }

        // Clear CCF flag
        self.aes.aes.cr.modify(|_, w| w.ccfc().set_bit());

        output
    }

    /// Finish the CTR stream
    ///
    /// Consumes this AES instance and returns another one that is back to the
    /// original state.
    pub fn finish(self) -> AES {
        // Disable AES
        self.aes.aes.cr.modify(|_, w| w.en().clear_bit());

        self.aes
    }
}


/// A 128-bit block
///
/// The AES peripheral processes 128 bits at a time, so this represents one unit
/// of processing.
pub type Block = [u8; 16];
