stm32l0xx-hal
=============

[![Build Status](https://travis-ci.com/stm32-rs/stm32l0xx-hal.svg?branch=master)](https://travis-ci.com/stm32-rs/stm32l0xx-hal)

WORK IN PROGRESS

[_stm32l0xx-hal_](https://github.com/stm32-rs/stm32l0xx-hal) is a Hardware Abstraction Layer (HAL) for the STMicro STM32L0xx family of microcontrollers.

This crate relies on Adam Greig's [stm32l0](https://crates.io/crates/stm32l0) crate to provide appropriate register definitions and implements a partial set of the [embedded-hal](https://github.com/rust-embedded/embedded-hal) traits.

Based on the [stm32l1xx-hal](https://github.com/stm32-rs/stm32l1xx-hal) crate by Vitaly Domnikov and the [stm32f4xx-hal](https://github.com/stm32-rs/stm32f4xx-hal) crate by Daniel Egger.


Supported Configurations
------------------------

* __stm32l0x1__

Dependencies
---------

1. Rustup toolchain installer

    https://rustup.rs


Configure Toolchain
---------

`$ rustup target add thumbv6m-none-eabi`

Build Examples
---------

`$ cargo build --release --examples --features stm32l0x1,rt`

Dependecies for Flashing
---------

1. Download and install the arm-none-eabi gcc toolchain

	https://developer.arm.com/open-source/gnu-toolchain/gnu-rm/downloads
	We recommend installing the precompiled binaries to '/usr/local'. 
	Add the bin folders (/bin & /arm-none-eabi/bin) to your environments variable 'PATH'.

2. Install STLink Tool

	https://github.com/texane/stlink

3. Install OpenOCD

    http://openocd.org/getting-openocd/ 

4. Install GDB Dashboard (OPTIONAL)

	https://github.com/cyrus-and/gdb-dashboard

Flashing
---------

The following is a how-to on flashing the 'serial' example code. This can be extended to any other example code.

1. Generate the hex file
    ``` 
    $ arm-none-eabi-objcopy -O ihex target/thumbv6m-none-eabi/release/examples/serial serial.hex
    ```

2. Flash the microcontroller
    ``` 
    $ st-flash --format ihex write serial.hex
    ```

Contibutor Notes
---------

- Revert local dependencies to external cargo and uncomment configurations before committing

License
-------

0-Clause BSD License, see [LICENSE-0BSD.txt](LICENSE-0BSD.txt) for more details.
