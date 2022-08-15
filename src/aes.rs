//! Interface to the AES peripheral.
//!
//! Note that the AES peripheral is only available on some MCUs in the L0/L1/L2
//! families. Check the datasheet for more information.
//!
//! See STM32L0x2 reference manual, chapter 18.

use core::{
    convert::TryInto,
    ops::{Deref, DerefMut},
    pin::Pin,
};

use as_slice::{AsMutSlice, AsSlice};
use nb::block;
use void::Void;

use crate::{
    dma,
    pac::{
        self,
        aes::{self, cr},
    },
    rcc::{Enable, Rcc, Reset},
};

/// Entry point to the AES API
pub struct AES {
    aes: pac::AES,
}

impl AES {
    /// Initialize the AES peripheral
    pub fn new(aes: pac::AES, rcc: &mut Rcc) -> Self {
        // Enable peripheral clock
        pac::AES::enable(rcc);
        // Reset peripheral
        pac::AES::reset(rcc);

        // Configure peripheral
        aes.cr.write(|w| {
            // Enable DMA
            w.dmaouten().set_bit();
            w.dmainen().set_bit();
            // Disable interrupts
            w.errie().clear_bit();
            w.ccfie().clear_bit()
        });

        Self { aes }
    }

    /// Enable the AES peripheral
    ///
    /// Returns a [`Stream`] instance which can be used to encrypt or decrypt
    /// data using the mode selected with the `mode` argument.
    ///
    /// Consumes the `AES` instance. You can get it back later once you're done
    /// with the `Stream`, using [`Stream::disable`].
    pub fn enable<M>(self, mode: M, key: [u32; 4]) -> Stream
    where
        M: Mode,
    {
        // Write key. This is safe, as the register accepts the full range of
        // `u32`.
        self.aes.keyr0.write(|w| w.bits(key[0]));
        self.aes.keyr1.write(|w| w.bits(key[1]));
        self.aes.keyr2.write(|w| w.bits(key[2]));
        self.aes.keyr3.write(|w| w.bits(key[3]));

        mode.prepare(&self.aes);

        self.aes.cr.modify(|_, w| {
            // Select mode
            mode.select(w);

            // Configure for stream of bytes
            // Safe, as we write a valid byte pattern.
            w.datatype().bits(0b10);

            // Enable peripheral
            w.en().set_bit()
        });

        Stream {
            aes: self,
            rx: Rx(()),
            tx: Tx(()),
        }
    }
}

/// An active encryption/decryption stream
///
/// You can get an instance of this struct by calling [`AES::enable`].
pub struct Stream {
    aes: AES,

    /// Can be used to write data to the AES peripheral
    pub tx: Tx,

    /// Can be used to read data from the AES peripheral
    pub rx: Rx,
}

impl Stream {
    /// Processes one block of data
    ///
    /// Writes one block of data to the AES peripheral, wait until it is
    /// processed then reads the processed block and returns it.
    ///
    /// Whether this is encryption or decryption depends on the mode that was
    /// selected when this `Stream` was created.
    pub fn process(&mut self, input: &Block) -> Result<Block, Error> {
        self.tx.write(input)?;
        // Can't panic. Error value of `Rx::read` is `Void`.
        let output = block!(self.rx.read()).unwrap();
        Ok(output)
    }

    /// Disable the AES peripheral
    ///
    /// Consumes the stream and returns the disabled [`AES`] instance. Call this
    /// method when you're done encrypting/decrypting data. You can then create
    /// another `Stream` using [`AES::enable`].
    pub fn disable(self) -> AES {
        // Disable AES
        self.aes.aes.cr.modify(|_, w| w.en().clear_bit());

        self.aes
    }
}

/// Can be used to write data to the AES peripheral
///
/// You can access this struct via [`Stream`].
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
        for i in (0..4).rev() {
            dinr.write(|w| {
                let i = i * 4;

                let word = &block[i..i + 4];
                // Can't panic, because `word` is 4 bytes long.
                let word = word.try_into().unwrap();
                let word = u32::from_le_bytes(word);

                w.bits(word)
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
    /// length must be a multiple of 16. Panics, if this is not the case.
    ///
    /// Panics, if the buffer is not aligned to a word boundary.
    pub fn write_all<Buffer, Channel>(
        self,
        dma: &mut dma::Handle,
        buffer: Pin<Buffer>,
        channel: Channel,
    ) -> Transfer<Self, Channel, Buffer, dma::Ready>
    where
        Self: dma::Target<Channel>,
        Buffer: Deref + 'static,
        Buffer::Target: AsSlice<Element = u8>,
        Channel: dma::Channel,
    {
        assert!(buffer.as_slice().len() % 16 == 0);

        // Safe, because we're only taking the address of a register.
        let address = &unsafe { &*pac::AES::ptr() }.dinr as *const _ as u32;

        // Safe, because the traits bounds of this method guarantee that
        // `buffer` can be read from.
        unsafe {
            Transfer::new(
                dma,
                self,
                channel,
                buffer,
                address,
                // This priority should be lower than the priority of the
                // transfer created in `read_all`. I'm not sure how relevant
                // that is in practice, but it makes sense, and as I've seen a
                // comment to that effect in ST's HAL code, I'd rather be
                // careful than risk weird bugs.
                dma::Priority::high(),
                dma::Direction::memory_to_peripheral(),
            )
        }
    }
}

/// Can be used to read data from the AES peripheral
///
/// You can access this struct via [`Stream`].
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
        for i in (0..4).rev() {
            let i = i * 4;

            let word = doutr.read().bits();
            let word = word.to_le_bytes();

            (&mut block[i..i + 4]).copy_from_slice(&word);
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
    /// length must be a multiple of 16. Panics, if this is not the case.
    ///
    /// Panics, if the buffer is not aligned to a word boundary.
    pub fn read_all<Buffer, Channel>(
        self,
        dma: &mut dma::Handle,
        buffer: Pin<Buffer>,
        channel: Channel,
    ) -> Transfer<Self, Channel, Buffer, dma::Ready>
    where
        Self: dma::Target<Channel>,
        Buffer: DerefMut + 'static,
        Buffer::Target: AsMutSlice<Element = u8>,
        Channel: dma::Channel,
    {
        assert!(buffer.as_slice().len() % 16 == 0);

        // Safe, because we're only taking the address of a register.
        let address = &unsafe { &*pac::AES::ptr() }.doutr as *const _ as u32;

        // Safe, because the traits bounds of this method guarantee that
        // `buffer` can be written to.
        unsafe {
            Transfer::new(
                dma,
                self,
                channel,
                buffer,
                address,
                // This priority should be higher than the priority of the
                // transfer created in `write_all`. I'm not sure how relevant
                // that is in practice, but it makes sense, and as I've seen a
                // comment to that effect in ST's HAL code, I'd rather be
                // careful than risk weird bugs.
                dma::Priority::very_high(),
                dma::Direction::peripheral_to_memory(),
            )
        }
    }
}

/// Implemented for all chaining modes
///
/// This is mostly an internal trait. The user won't typically need to use or
/// implement this, except to call the various static methods that create a
/// mode.
pub trait Mode {
    fn prepare(&self, _: &aes::RegisterBlock);
    fn select(&self, _: &mut cr::W);
}

impl dyn Mode {
    /// Use this with [`AES::enable`] to encrypt using ECB
    pub fn ecb_encrypt() -> ECB<Encrypt> {
        ECB(Encrypt)
    }

    /// Use this with [`AES::enable`] to decrypt using ECB
    pub fn ecb_decrypt() -> ECB<Decrypt> {
        ECB(Decrypt)
    }

    /// Use this with [`AES::enable`] to encrypt using CBC
    pub fn cbc_encrypt(init_vector: [u32; 4]) -> CBC<Encrypt> {
        CBC {
            _mode: Encrypt,
            init_vector,
        }
    }

    /// Use this with [`AES::enable`] to decrypt using CBC
    pub fn cbc_decrypt(init_vector: [u32; 4]) -> CBC<Decrypt> {
        CBC {
            _mode: Decrypt,
            init_vector,
        }
    }

    /// Use this with [`AES::enable`] to encrypt or decrypt using CTR
    pub fn ctr(init_vector: [u32; 3]) -> CTR {
        CTR { init_vector }
    }
}

/// The ECB (electronic code book) chaining mode
///
/// Can be passed [`AES::enable`], to start encrypting or decrypting using ECB
/// mode. `Mode` must be either [`Encrypt`] or [`Decrypt`].
///
/// You gen get an instance of this struct via [`Mode::ecb_encrypt`] or
/// [`Mode::ecb_decrypt`].
pub struct ECB<Mode>(Mode);

impl Mode for ECB<Encrypt> {
    fn prepare(&self, _: &aes::RegisterBlock) {
        // Nothing to do.
    }

    fn select(&self, w: &mut cr::W) {
        // Safe, as we're only writing valid bit patterns.
        unsafe {
            // Select ECB chaining mode
            w.chmod().bits(0b00);
            // Select encryption mode
            w.mode().bits(0b00);
        }
    }
}

impl Mode for ECB<Decrypt> {
    fn prepare(&self, aes: &aes::RegisterBlock) {
        derive_key(aes)
    }

    fn select(&self, w: &mut cr::W) {
        // Safe, as we're only writing valid bit patterns.
        unsafe {
            // Select ECB chaining mode
            w.chmod().bits(0b00);
            // Select decryption mode
            w.mode().bits(0b10);
        }
    }
}

/// The CBC (cipher block chaining) chaining mode
///
/// Can be passed [`AES::enable`], to start encrypting or decrypting using CBC
/// mode. `Mode` must be either [`Encrypt`] or [`Decrypt`].
///
/// You gen get an instance of this struct via [`Mode::cbc_encrypt`] or
/// [`Mode::cbc_decrypt`].
pub struct CBC<Mode> {
    _mode: Mode,
    init_vector: [u32; 4],
}

impl Mode for CBC<Encrypt> {
    fn prepare(&self, aes: &aes::RegisterBlock) {
        // Safe, as the registers accept the full range of `u32`.
        aes.ivr3.write(|w| w.bits(self.init_vector[0]));
        aes.ivr2.write(|w| w.bits(self.init_vector[1]));
        aes.ivr1.write(|w| w.bits(self.init_vector[2]));
        aes.ivr0.write(|w| w.bits(self.init_vector[3]));
    }

    fn select(&self, w: &mut cr::W) {
        // Safe, as we're only writing valid bit patterns.
        unsafe {
            // Select CBC chaining mode
            w.chmod().bits(0b01);
            // Select encryption mode
            w.mode().bits(0b00);
        }
    }
}

impl Mode for CBC<Decrypt> {
    fn prepare(&self, aes: &aes::RegisterBlock) {
        derive_key(aes);

        // Safe, as the registers accept the full range of `u32`.
        aes.ivr3.write(|w| w.bits(self.init_vector[0]));
        aes.ivr2.write(|w| w.bits(self.init_vector[1]));
        aes.ivr1.write(|w| w.bits(self.init_vector[2]));
        aes.ivr0.write(|w| w.bits(self.init_vector[3]));
    }

    fn select(&self, w: &mut cr::W) {
        // Safe, as we're only writing valid bit patterns.
        unsafe {
            // Select CBC chaining mode
            w.chmod().bits(0b01);
            // Select decryption mode
            w.mode().bits(0b10);
        }
    }
}

/// The CTR (counter) chaining mode
///
/// Can be passed [`AES::enable`], to start encrypting or decrypting using CTR
/// mode. In CTR mode, encryption and decryption are technically identical, so
/// further qualification is not required.
///
/// You gen get an instance of this struct via [`Mode::ctr`].
pub struct CTR {
    init_vector: [u32; 3],
}

impl Mode for CTR {
    fn prepare(&self, aes: &aes::RegisterBlock) {
        // Initialize initialization vector
        //
        // See STM32L0x2 reference manual, table 78 on page 408.
        aes.ivr3.write(|w| w.bits(self.init_vector[0]));
        aes.ivr2.write(|w| w.bits(self.init_vector[1]));
        aes.ivr1.write(|w| w.bits(self.init_vector[2]));
        aes.ivr0.write(|w| w.bits(0x0001)); // counter
    }

    fn select(&self, w: &mut cr::W) {
        // Safe, as we're only writing valid bit patterns.
        unsafe {
            // Select Counter Mode (CTR) mode
            w.chmod().bits(0b10);
            // These bits mean encryption mode, but in CTR mode,
            // encryption and descryption are technically identical, so this
            // is fine for either mode.
            w.mode().bits(0b00);
        }
    }
}

fn derive_key(aes: &aes::RegisterBlock) {
    // Select key derivation mode. This is safe, as we're writing a valid bit
    // pattern.
    aes.cr.modify(|_, w| w.mode().bits(0b01));

    // Enable the peripheral. It will be automatically disabled again once the
    // key has been derived.
    aes.cr.modify(|_, w| w.en().set_bit());

    // Wait for key derivation to finish
    while aes.sr.read().ccf().bit_is_clear() {}
}

/// Used to identify encryption mode
pub struct Encrypt;

/// Used to identify decryption mode
pub struct Decrypt;

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

/// Wrapper around a [`dma::Transfer`].
///
/// This struct is required, because under the hood, the AES peripheral only
/// supports 32-bit word DMA transfers, while the public API works with byte
/// slices.
pub struct Transfer<Target, Channel, Buffer, State> {
    buffer: Pin<Buffer>,
    inner: dma::Transfer<Target, Channel, dma::PtrBuffer<u32>, State>,
}

impl<Target, Channel, Buffer> Transfer<Target, Channel, Buffer, dma::Ready>
where
    Target: dma::Target<Channel>,
    Channel: dma::Channel,
    Buffer: Deref + 'static,
    Buffer::Target: AsSlice<Element = u8>,
{
    /// Create a new instance of `Transfer`
    ///
    /// # Safety
    ///
    /// If this is used to prepare a memory-to-peripheral transfer, the caller
    /// must make sure that the buffer can be read from safely.
    ///
    /// If this is used to prepare a peripheral-to-memory transfer, the caller
    /// must make sure that the buffer can be written to safely.
    ///
    /// The caller must guarantee that the buffer length is a multiple of 4.
    unsafe fn new(
        dma: &mut dma::Handle,
        target: Target,
        channel: Channel,
        buffer: Pin<Buffer>,
        address: u32,
        priority: dma::Priority,
        dir: dma::Direction,
    ) -> Self {
        let num_words = buffer.as_slice().len() / 4;

        let transfer = dma::Transfer::new(
            dma,
            target,
            channel,
            // The caller must guarantee that our length is a multiple of 4, so
            // this should be fine.
            Pin::new(dma::PtrBuffer {
                ptr: buffer.as_slice().as_ptr() as *const u32,
                len: num_words,
            }),
            num_words,
            address,
            priority,
            dir,
            false,
        );

        Self {
            buffer,
            inner: transfer,
        }
    }

    /// Enables the provided interrupts
    ///
    /// This setting only affects this transfer. It doesn't affect transfer on
    /// other channels, or subsequent transfers on the same channel.
    pub fn enable_interrupts(&mut self, interrupts: dma::Interrupts) {
        self.inner.enable_interrupts(interrupts)
    }

    /// Start the DMA transfer
    ///
    /// Consumes this instance of `Transfer` and returns a new one, with its
    /// state changes to indicate that the transfer has been started.
    pub fn start(self) -> Transfer<Target, Channel, Buffer, dma::Started> {
        Transfer {
            buffer: self.buffer,
            inner: self.inner.start(),
        }
    }
}

impl<Target, Channel, Buffer> Transfer<Target, Channel, Buffer, dma::Started>
where
    Channel: dma::Channel,
{
    /// Indicates whether the transfer is still ongoing
    pub fn is_active(&self) -> bool {
        self.inner.is_active()
    }

    /// Waits for the transfer to finish and returns the owned resources
    ///
    /// This function will busily wait until the transfer is finished. If you
    /// don't want this, please call this function only once you know that the
    /// transfer has finished.
    ///
    /// This function will return immediately, if [`Transfer::is_active`]
    /// returns `false`.
    pub fn wait(self) -> dma::TransferResourcesResult<Target, Channel, Buffer> {
        let (res, err) = match self.inner.wait() {
            Ok(res) => (res, None),
            Err((res, err)) => (res, Some(err)),
        };

        let res = dma::TransferResources {
            target: res.target,
            channel: res.channel,
            buffer: self.buffer,
        };

        match err {
            None => Ok(res),
            Some(err) => Err((res, err)),
        }
    }
}
