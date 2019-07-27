//! Interface to the AES peripheral
//!
//! See STM32L0x2 reference manual, chapter 18.


use core::{
    convert::TryInto,
    ops::{
        Deref,
        DerefMut,
    },
    pin::Pin,
};

use as_slice::{
    AsMutSlice,
    AsSlice,
};
use nb::block;
use void::Void;

use crate::{
    dma,
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
                // Enable DMA
                .dmaouten().set_bit()
                .dmainen().set_bit()
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
            rx:  Rx(()),
            tx:  Tx(()),
        }
    }
}


/// An active encryption/decryption stream using CTR mode
///
/// You can get an instance of this struct by calling [`AES::start_ctr_stream`].
pub struct CtrStream {
    aes: AES,

    pub tx: Tx,
    pub rx: Rx,
}

impl CtrStream {
    /// Processes one block of data
    ///
    /// In CTR mode, encrypting and decrypting work the same. If you pass a
    /// block of clear data to this function, an encrypted block is returned. If
    /// you pass a block of encrypted data, it is decrypted and a clear block
    /// is returned.
    pub fn process(&mut self, input: &Block) -> Result<Block, Error> {
        self.tx.write(input)?;
        // Can't panic. Error value of `Rx::read` is `Void`.
        let output = block!(self.rx.read()).unwrap();
        Ok(output)
    }

    /// Finish the CTR stream
    ///
    /// Consumes the stream and returns the AES peripheral that was used to
    /// start it.
    pub fn finish(self) -> AES {
        // Disable AES
        self.aes.aes.cr.modify(|_, w| w.en().clear_bit());

        self.aes
    }
}


/// Can be used to write data to the AES peripheral
///
/// You can access this struct via [`CtrStream`].
pub struct Tx(());

impl Tx {
    /// Write a block to the AES peripheral
    ///
    /// Please note that only one block can be written before you need to read
    /// the processed block back using [`Read::read`]. Calling this method
    /// multiple times without calling [`Read::read`] in between will result in
    /// an error to be returned.
    pub fn write(&mut self, block: &Block) -> Result<(), Error> {
        // Get access to the registers. This is safe, because:
        // - `Tx` has exclusive access to DINR.
        // - We only use SR for an atomic read.
        let (dinr, sr) = unsafe {
            let aes = &*pac::AES::ptr();
            (&aes.dinr, &aes.sr)
        };

        // Write input data to DINR
        //
        // See STM32L0x2 reference manual, section 18.4.10.
        for i in (0 .. 4).rev() {
            dinr.write(|w| {
                let i = i * 4;

                let word = &block[i .. i+4];
                // Can't panic, because `word` is 4 bytes long.
                let word = word.try_into().unwrap();
                let word = u32::from_le_bytes(word);

                unsafe { w.bits(word) }
            });
        }

        // Was there an unexpected write? If so, a computation is already
        // ongoing and the user needs to call `Rx::read` next. If I understand
        // the documentation correctly, our writes to the register above
        // shouldn't have affected the ongoing computation.
        if sr.read().wrerr().bit_is_set() {
            return Err(Error::Busy);
        }

        Ok(())
    }

    /// Writes the provided buffer to the AES peripheral using DMA
    ///
    /// Returns a DMA transfer that is ready to be started. It needs to be
    /// started for anything to happen.
    ///
    /// # Panics
    ///
    /// Panics, if the buffer length is larger than `u16::max_value()`.
    ///
    /// The AES peripheral works with 128-bit blocks, which means the buffer
    /// length must be a multiple of 4. Panics, if this is not the case.
    pub fn write_all<Buffer, Channel>(self,
        dma:     &mut dma::Handle,
        buffer:  Pin<Buffer>,
        channel: Channel,
    )
        -> dma::Transfer<Self, Channel, Buffer, dma::Ready>
        where
            Self:           dma::Target<Channel>,
            Buffer:         Deref + 'static,
            Buffer::Target: AsSlice<Element=u32>,
            Channel:        dma::Channel,
    {
        assert!(buffer.as_slice().len() % 4 == 0);

        // Safe, because we're only taking the address of a register.
        let address = &unsafe { &*pac::AES::ptr() }.dinr as *const _ as u32;

        dma::Transfer::memory_to_peripheral(dma, self, channel, buffer, address)
    }
}


/// Can be used to read data from the AES peripheral
///
/// You can access this struct via [`CtrStream`].
pub struct Rx(());

impl Rx {
    pub fn read(&mut self) -> nb::Result<Block, Void> {
        // Get access to the registers. This is safe, because:
        // - We only use SR for an atomic read.
        // - `Rx` has exclusive access to DOUTR.
        // - While it exists, `Rx` has exlusive access to CR.
        let (sr, doutr, cr) = unsafe {
            let aes = &*pac::AES::ptr();
            (&aes.sr, &aes.doutr, &aes.cr)
        };

        // Is a computation complete?
        if sr.read().ccf().bit_is_clear() {
            return Err(nb::Error::WouldBlock);
        }

        // Read output data from DOUTR
        //
        // See STM32L0x2 reference manual, section 18.4.10.
        let mut block = [0; 16];
        for i in (0 .. 4).rev() {
            let i = i * 4;

            let word = doutr.read().bits();
            let word = word.to_le_bytes();

            (&mut block[i .. i+4]).copy_from_slice(&word);
        }

        // Clear CCF flag
        cr.modify(|_, w| w.ccfc().set_bit());

        Ok(block)
    }

    /// Reads data from the AES peripheral into the provided buffer using DMA
    ///
    /// Returns a DMA transfer that is ready to be started. It needs to be
    /// started for anything to happen.
    ///
    /// # Panics
    ///
    /// Panics, if the buffer length is larger than `u16::max_value()`.
    ///
    /// The AES peripheral works with 128-bit blocks, which means the buffer
    /// length must be a multiple of 4. Panics, if this is not the case.
    pub fn read_all<Buffer, Channel>(self,
        dma:     &mut dma::Handle,
        buffer:  Pin<Buffer>,
        channel: Channel,
    )
        -> dma::Transfer<Self, Channel, Buffer, dma::Ready>
        where
            Self:           dma::Target<Channel>,
            Buffer:         DerefMut + 'static,
            Buffer::Target: AsMutSlice<Element=u32>,
            Channel:        dma::Channel,
    {
        assert!(buffer.as_slice().len() % 4 == 0);

        // Safe, because we're only taking the address of a register.
        let address = &unsafe { &*pac::AES::ptr() }.doutr as *const _ as u32;

        dma::Transfer::peripheral_to_memory(dma, self, channel, buffer, address)
    }
}


/// A 128-bit block
///
/// The AES peripheral processes 128 bits at a time, so this represents one unit
/// of processing.
pub type Block = [u8; 16];


#[derive(Debug)]
pub enum Error {
    /// AES peripheral is busy
    Busy,
}
