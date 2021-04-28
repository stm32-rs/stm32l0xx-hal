//! Factory-programmed data
//!
//! The STM32L0 contains a few read-only registers with factory programming
//! data that are written during production.

#![no_std]

use cortex_m::interrupt;

mod pointers {
    pub const DEVICE_ID_PTR: *const u8 = 0x1FF8_0050 as _;
    pub const FLASH_SIZE_PTR: *const u16 = 0x1FF8_007C as _;
}

use pointers::*;

/// Returns a 12-byte unique device ID
pub fn device_id() -> &'static [u8; 12] {
    unsafe { &*DEVICE_ID_PTR.cast::<[u8; 12]>() }
}

/// Returns a string with a hex-encoded unique device ID
pub fn device_id_hex() -> &'static str {
    static mut DEVICE_ID_STR: [u8; 24] = [0; 24];

    unsafe {
        if DEVICE_ID_STR.as_ptr().read_volatile() == 0 {
            interrupt::free(|_| {
                let hex = b"0123456789abcdef";
                for (i, b) in device_id().iter().enumerate() {
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
