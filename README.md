stm32l0xx-hal
=============

[_stm32l0xx-hal_](https://github.com/stm32-rs/stm32l0xx-hal) is a Hardware Abstraction Layer (HAL) for the STMicro STM32L0xx family of microcontrollers.

This crate relies on Adam Greig's [stm32l0](https://crates.io/crates/stm32l0) crate to provide appropriate register definitions and implements a partial set of the [embedded-hal](https://github.com/rust-embedded/embedded-hal) traits.

Based on the [stm32l1xx-hal](https://github.com/stm32-rs/stm32l1xx-hal) crate by Vitaly Domnikov and [stm32f4xx-hal](https://github.com/stm32-rs/stm32f4xx-hal) crate by Daniel Egger.


Supported Configurations
------------------------

* __stm32l0x1__


Check HAL Instructions
---------

`$ rustup default nightly`

`$ export COMMAND=""`

`$ rustup target add thumbv6m-none-eabi`

`$ tools/check.py $COMMAND`

Build Examples
---------

`$ cargo build --examples --features stm32l011`

Contibutor Notes
---------

- Revert local dependencies to external cargo and uncomment configurations before committing

License
-------

0-Clause BSD License, see [LICENSE-0BSD.txt](LICENSE-0BSD.txt) for more details.