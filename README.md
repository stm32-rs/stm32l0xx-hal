stm32l0xx-hal
=============

[![Build Status](https://travis-ci.com/stm32-rs/stm32l0xx-hal.svg?branch=master)](https://travis-ci.com/stm32-rs/stm32l0xx-hal)

[_stm32l0xx-hal_](https://github.com/stm32-rs/stm32l0xx-hal) is a Hardware Abstraction Layer (HAL) for the STMicro STM32L0xx family of microcontrollers.

This crate relies on Adam Greig's [stm32l0](https://crates.io/crates/stm32l0) crate to provide appropriate register definitions and implements a partial set of the [embedded-hal](https://github.com/rust-embedded/embedded-hal) traits.

Based on the [stm32l1xx-hal](https://github.com/stm32-rs/stm32l1xx-hal) crate by Vitaly Domnikov and the [stm32f4xx-hal](https://github.com/stm32-rs/stm32f4xx-hal) crate by Daniel Egger.


# Supported Configurations

* __stm32l0x1__
* __stm32l0x2__
* __stm32l0x3__

# Build Dependencies

1. Rustup toolchain installer

    https://rustup.rs


# Toolchain Configuration

`$ rustup target add thumbv6m-none-eabi`

# Build Examples

`$ cargo build --release --examples --features stm32l0x1,rt`

# Using as a Dependency

To use the stm32l0xx-hal [crate](https://crates.io/crates/stm32l0xx-hal) as a dependency, add the following definition to your `Cargo.toml`:

``` 
[dependencies.stm32l0xx-hal]
version = "0.6.2"
features = ["stm32l0x1", "rt"]
```

Example Projects: [HABEXpico](https://github.com/arkorobotics/HABEXpico/tree/master/Firmware)

# Dependecies for Flashing

1. Download and install the arm-none-eabi toolchain

	https://developer.arm.com/open-source/gnu-toolchain/gnu-rm/downloads
	We recommend installing the precompiled binaries to '/usr/local/opt'. 
	Add the bin folders (/bin & /arm-none-eabi/bin) to your environments variable 'PATH'.

2. Install STLink Tool (>=v1.5.1)

	https://github.com/texane/stlink

3. Install OpenOCD (OPTIONAL)

    NOTE: OpenOCD v0.10.0 does not fully support the stm32l0 family MCU. We recommend using `gnu-mcu-eclipse/openocd` instead:

    https://gnu-mcu-eclipse.github.io/openocd/install/
    We recommend installing the precompiled binaries to '/usr/local/opt'. 
	Add the bin folders (i.e. - /usr/local/opt/gnu-mcu-eclipse/openocd/0.10.0-12-20190422-2015/bin) to your environments variable 'PATH'.

4. Install GDB Dashboard (OPTIONAL)

	https://github.com/cyrus-and/gdb-dashboard

# Flashing

The following instructions outline how-to on flashing the 'serial' example code. This can be extended to any other example code.

## Flashing with ST-Flash:

1. Flash the microcontroller using the flash script
    ``` 
    $ ./flash.sh target/thumbv6m-none-eabi/release/examples/serial
    ```

## Flashing with OpenOCD

1. Flash the microcontroller using the openocd flash script
    ``` 
    $ ./openocd_flash.sh target/thumbv6m-none-eabi/release/examples/serial
    ```

# Debugging

## Debugging with GDB

1. Terminal 1 - OpenOCD Session:
    ``` 
    $ ./openocd_session.sh
    ```
    
2. Terminal 2 - GDB Session:
    ``` 
    $ ./gdb_session.sh target/thumbv6m-none-eabi/release/examples/serial
    ```

## Debugging with GDB Py and GDB Dashboard

1. Terminal 1 - OpenOCD Session:
    ``` 
    $ ./openocd_session.sh
    ```

2. Terminal 2 - GDB Py Session:
    ``` 
    $ ./gdb_session.sh target/thumbv6m-none-eabi/release/examples/serial -d
    ```

    Note: Users can redirect the dashboard output to separate terminal (i.e. - ttys001) using:
    ```
    >>> dashboard -output /dev/ttys001
    ```

Contibutor Notes
---------

- Revert local dependencies to external cargo and uncomment configurations before committing

License
-------

0-Clause BSD License, see [LICENSE-0BSD.txt](LICENSE-0BSD.txt) for more details.
