//! Factory-programmed data
//!
//! The STM32L0 contains a few read-only registers with factory programming
//! data that are written during production.

use cortex_m::interrupt;

mod pointers {
    pub const FACTORY_PROD_PTR: *const u8 = 0x1FF8_0050 as _;
    pub const UNIQUE_ID_PTR: *const u8 = 0x1FF8_0064 as _;
    pub const FLASH_SIZE_PTR: *const u16 = 0x1FF8_007C as _;
}

use pointers::*;

/// Returns a 12-byte unique device ID
/// According to the Reference Manual, the device electronic
/// signature is non-contiguous. This wrapper makes it contiguous
/// since the unicity is only given by all of those fields.
pub fn device_id(buffer: &mut [u8; 12]) {
    unsafe {
        buffer[0..8].copy_from_slice(&*(FACTORY_PROD_PTR).cast::<[u8; 8]>());
        buffer[8..12].copy_from_slice(&*(UNIQUE_ID_PTR).cast::<[u8; 4]>());
    }
}

/// Returns a string with a hex-encoded unique device ID
pub fn device_id_hex() -> &'static str {
    static mut DEVICE_ID_STR: [u8; 24] = [0; 24];
    let mut buffer: [u8; 12] = [0; 12];
    device_id(&mut buffer);
    unsafe {
        if DEVICE_ID_STR.as_ptr().read_volatile() == 0 {
            interrupt::free(|_| {
                let hex = b"0123456789abcdef";
                for (i, b) in buffer.iter().enumerate() {
                    let lo = b & 0xf;
                    let hi = (b >> 4) & 0xfu8;
                    DEVICE_ID_STR[i * 2] = hex[hi as usize];
                    DEVICE_ID_STR[i * 2 + 1] = hex[lo as usize];
                }
            });
        }

        core::str::from_utf8_unchecked(&DEVICE_ID_STR)
    }
}

/// Returns the Flash memory size of the device in Kbytes
pub fn flash_size_kb() -> u16 {
    unsafe { *FLASH_SIZE_PTR }
}
