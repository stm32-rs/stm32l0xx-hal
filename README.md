stm32l0xx-hal
=============

[![Build Status](https://travis-ci.com/stm32-rs/stm32l0xx-hal.svg?branch=master)](https://travis-ci.com/stm32-rs/stm32l0xx-hal)

[_stm32l0xx-hal_](https://github.com/stm32-rs/stm32l0xx-hal) is a Hardware
Abstraction Layer (HAL) for the STMicro STM32L0xx family of microcontrollers.

This crate relies on Adam Greig's [stm32l0](https://crates.io/crates/stm32l0)
crate to provide appropriate register definitions and implements a partial set
of the [embedded-hal](https://github.com/rust-embedded/embedded-hal) traits.

Based on the [stm32l1xx-hal](https://github.com/stm32-rs/stm32l1xx-hal) crate
by Vitaly Domnikov and the [stm32f4xx-hal](https://github.com/stm32-rs/stm32f4xx-hal)
crate by Daniel Egger.


# Usage

Add the [`stm32l0xx-hal` crate](https://crates.io/crates/stm32l0xx-hal) to your
dependencies in `Cargo.toml` and make sure to pick the appropriate `mcu-*`
Cargo feature to enjoy the full feature set for your MCU (see next section
"Supported Configurations" for more details).

For example, when using the STM32L071KBTx MCU:

```toml
[dependencies]
stm32l0xx-hal = { version = "0.7.0", features = ["mcu-STM32L071KBTx", "rt"] }
```


# Supported Configurations

The STM32L0 family consists of different subfamilies with different peripherals
and I/O configurations. Superficially, the family can be grouped into the
groups `stm32l0x1`, `stm32l0x2` and `stm32l0x3`. However, some aspects like
alternate function mappings for I/O pins do not follow these groups.

In order for the HAL to properly support all those MCUs, we generate some
peripheral mappings and corresponding Cargo features using
[cube-parse](https://github.com/dbrgn/cube-parse/).

## MCU Features (`mcu-*`)

The easiest way for you to get started, is to use your appropriate `mcu-*`
feature. For example, when using the STM32L071KBTx MCU, you just set the
`mcu-STM32L071KBTx` feature in `Cargo.toml`:

```toml
# Cargo.toml
[dependencies]
stm32l0xx-hal = { version = "0.7.0", features = ["mcu-STM32L071KBTx", "rt"] }
```

If you take a look at the [`Cargo.toml`
file](https://github.com/stm32-rs/stm32l0xx-hal/blob/master/Cargo.toml), you
can see that `mcu-STM32L071KBTx` is just an alias for `["io-STM32L071",
"stm32l0x1", "lqfp32"]`.

## I/O Features (`io-*`)

The `io-*` features are based on the GPIO peripheral version. This determines
the pin function mapping of the MCU. The features seem to correspond to the
product categories.

Right now, the following features are supported:

- `io-STM32L021` (Product category 1)
- `io-STM32L031` (Product category 2)
- `io-STM32L051` (Product category 3)
- `io-STM32L071` (Product category 5)

The product categories should be listed in your MCU family datasheet. The name
of the `io-*` feature itself is derived from the internal name used in the
STM32CubeMX database. It does not necessarily match the name of the MCU,
for example the `STM32L062K8Tx` uses the GPIO peripheral version named
`io-STM32L051`.

## Flash memory

Currently a default Flash and RAM size are configured depending on the chip's subfamily.

| Chip        | Flash size | RAM size |
| ----------- | ---------- | -------- |
| `stm32l0x1` | 16kB       | 2kB      |
| `stm32l0x2` | 192kB      | 20kB     |
| `stm32l0x3` | 64kB       | 8kB      |

However not all chips in the subfamily have the same flash and RAM size. Due to a known limitation
to the scripts used to generate the features the sizes might not correspond to the true sizes of 
the MCU. You should double check with the datasheet to make sure the flash and RAM sizes are correct.

If they are not correct, you can override the `memory.x` of `stm32l0xx-hal` by providing your own.
In your crate root, add a file called `memory.x` with the correct configuration. For example:

```
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 64K
  RAM : ORIGIN = 0x20000000, LENGTH = 8K
}
```

Add a `build.rs` file with the following content:

```rust
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put the linker script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=memory.x");
}
```

And finally add the `disable-linker-script` feature to your `stm32l0xx-hal` dependency:

```toml
# Cargo.toml
[dependencies]
stm32l0xx-hal = { version = "0.7.0", features = ["mcu-STM32L071K8Ux", "disable-linker-script"] }
```

For more information, see: https://github.com/stm32-rs/stm32l0xx-hal/issues/170


# Toolchain Setup

In order to use this HAL, you need the following Setup:

1. Install Rustup

    See [rustup.rs](https://rustup.rs/) for details. You may als be able to
    install Rustup directly through your distro.

2. Install the `arm-none-eabi` compiler toolchain

	https://developer.arm.com/open-source/gnu-toolchain/gnu-rm/downloads

    If you cannot install the toolchain directly through your OS / distro, we
    recommend installing the precompiled binaries to '/usr/local/opt'.  Add the
    bin folders (/bin & /arm-none-eabi/bin) to your environments variable 'PATH'.

3. Install the `thumbv6m-none-eabi` target for Rust

    Simply run `rustup target add thumbv6m-none-eabi`

For more instructions on how to get started with ARM / Cortex-M programming
using Rust, check out the [Embedded Rust
Book](https://rust-embedded.github.io/book/).


# Build Examples

You can build examples through Cargo:

    $ cargo build --release --examples --features stm32l0x1,rt

Note that not all examples are compatible with all MCUs. You might need to peek
into the example source code.


# Flashing Using Helper Scripts

The following instructions outline how-to on flashing the 'serial' example
code. This can be extended to any other example code.

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


# Debugging Using Helper Scripts

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


# Contibutor Notes

- Revert local dependencies to external cargo and uncomment configurations
  before committing


# License

0-Clause BSD License, see [LICENSE-0BSD.txt](LICENSE-0BSD.txt) for more details.
