stm32l0xx-hal
=============


WORK IN PROGRESS 
[![Build Status](https://travis-ci.com/stm32-rs/stm32l0xx-hal.svg?branch=master)](https://travis-ci.com/stm32-rs/stm32l0xx-hal)


[_stm32l0xx-hal_](https://github.com/stm32-rs/stm32l0xx-hal) is a Hardware Abstraction Layer (HAL) for the STMicro STM32L0xx family of microcontrollers.

This crate relies on Adam Greig's [stm32l0](https://crates.io/crates/stm32l0) crate to provide appropriate register definitions and implements a partial set of the [embedded-hal](https://github.com/rust-embedded/embedded-hal) traits.

Based on the [stm32l1xx-hal](https://github.com/stm32-rs/stm32l1xx-hal) crate by Vitaly Domnikov and [stm32f4xx-hal](https://github.com/stm32-rs/stm32f4xx-hal) crate by Daniel Egger.


Supported Configurations
------------------------

* __stm32l0x1__

Prerequisites for building local stm32-rs
---------

Requires svd2rust 0.14.0 or greater:

`$ cargo install svd2rust`

Build local stm32-rs 
---------

At the root of the stm32-rs directory

`$ make`

Check HAL Instructions
---------

`$ rustup target add thumbv6m-none-eabi`

`$ cargo check --features=stm32l0x1,rt`

Build Examples
---------

`$ cargo build --release --examples --features stm32l0x1,rt`


Flash
---------

The following is a how-to on flashing the 'serial' example code. This can be extended to any other example code.

1. Generate Hex File
``` 
$ arm-none-eabi-objcopy -O ihex target/thumbv6m-none-eabi/release/examples/serial serial.hex
```

2. Flash command
``` 
$ st-flash --format ihex write serial.hex
```

Contibutor Notes
---------

- Revert local dependencies to external cargo and uncomment configurations before committing

License
-------

0-Clause BSD License, see [LICENSE-0BSD.txt](LICENSE-0BSD.txt) for more details.
