# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/).



## [Unreleased]

<!-- When making a PR, please update this section. Note: This document should
make upgrading the HAL as painless as possible! If it makes sense, feel free
to add upgrade notes and examples. When adding an issue or PR reference, don't
forget to update the links at the bottom of the changelog as well.-->

### Additions

- Add `Timer::new` to create timer without starting #152

### Breaking Changes

- Add `enable` to `GeneralPurposeTimer`

### Non-Breaking Changes

### Fixes

### Documentation



## [v0.7.0] - 2021-03-10

### Additions

- Timers: Add support for TIM6 ([#101])
- Timers: Implement `LinkedTimer` ([#115]). It is initialized with two hardware
  timers (either TIM2/TIM3 or TIM21/TIM22). The two timers are configured in
  master/slave mode so that an overflow of the master timer triggers an update
  on the slave timer. This way, two 16 bit timers can be combined to a single
  32 bit timer.  (The STM32L0 does not have 32 bit timers.)
- Remove config tests that prevented stm32l0x1 devices from using ADC with DMA ([#124])
- Add `enable_lse` method to RCC structure that configures LSE ([#130])
- Expose factory calibration data ([#121])
- Prelude: add `Serial1Ext` ([#133])
- Flash: support stm32l0x3 ([#134])
- Add Cargo features for EEPROM size, generated with cube-parse ([#137])
- Flash: Support writing single bytes to EEPROM ([#140])
- Add MCO support ([#143])
- Timers: Add QEI support for LPTIM ([#144])
- Timers: Add encoder input support for TIM2 and TIM21 ([#145])
- Timers: Adds support for LSE clocking to LPUART ([#131])
- SPI: Add DMA support ([#148])

### Breaking Changes

### Non-Breaking Changes

- Improvements to polling i²c driver ([#102])
- Enforce rustfmt ([#107])
- Simplify `flash.c` ([#136])

### Fixes

- I2C: Before starting new transaction ensure that both TX and RX buffers are empty ([#98])
- Detect rollover using positions rather than DMA flags ([#127]). This fixes a
  faulty DMA buffer overflow in `adc_trig` example ([#104]).
- Maximum frequency for stm32l0 increased to 32MHz ([#129])
- GPIO: Fix `Pin::into_pull_down_input` ([#142])

### Documentation

- Document MSIRange enum variants ([#99])
- Port examples to RTFM/RTIC 0.5 ([#100], [#116])
- Improve README ([#119])



## [v0.6.2] - 2020-05-03

_Not yet tracked in this changelog._



## [v0.6.1] - 2020-04-08

_Not yet tracked in this changelog._



## [v0.6.0] - 2020-04-05

_Not yet tracked in this changelog._



## v0.5.0 - 2019-12-02

*Not yet tracked in this changelog.*



<!-- Links to pull requests and issues. Note that you can use "issues"
in the URL for both issues and pull requests. -->
[#148]: https://github.com/stm32-rs/stm32l0xx-hal/issues/148
[#145]: https://github.com/stm32-rs/stm32l0xx-hal/issues/145
[#144]: https://github.com/stm32-rs/stm32l0xx-hal/issues/144
[#143]: https://github.com/stm32-rs/stm32l0xx-hal/issues/143
[#142]: https://github.com/stm32-rs/stm32l0xx-hal/issues/142
[#140]: https://github.com/stm32-rs/stm32l0xx-hal/issues/140
[#137]: https://github.com/stm32-rs/stm32l0xx-hal/issues/137
[#136]: https://github.com/stm32-rs/stm32l0xx-hal/issues/136
[#134]: https://github.com/stm32-rs/stm32l0xx-hal/issues/134
[#133]: https://github.com/stm32-rs/stm32l0xx-hal/issues/133
[#131]: https://github.com/stm32-rs/stm32l0xx-hal/issues/131
[#130]: https://github.com/stm32-rs/stm32l0xx-hal/issues/130
[#129]: https://github.com/stm32-rs/stm32l0xx-hal/issues/129
[#127]: https://github.com/stm32-rs/stm32l0xx-hal/issues/127
[#124]: https://github.com/stm32-rs/stm32l0xx-hal/issues/124
[#121]: https://github.com/stm32-rs/stm32l0xx-hal/issues/121
[#119]: https://github.com/stm32-rs/stm32l0xx-hal/issues/119
[#116]: https://github.com/stm32-rs/stm32l0xx-hal/issues/116
[#115]: https://github.com/stm32-rs/stm32l0xx-hal/issues/115
[#107]: https://github.com/stm32-rs/stm32l0xx-hal/issues/107
[#104]: https://github.com/stm32-rs/stm32l0xx-hal/issues/104
[#102]: https://github.com/stm32-rs/stm32l0xx-hal/issues/102
[#101]: https://github.com/stm32-rs/stm32l0xx-hal/issues/101
[#100]: https://github.com/stm32-rs/stm32l0xx-hal/issues/100
[#99]: https://github.com/stm32-rs/stm32l0xx-hal/issues/99
[#98]: https://github.com/stm32-rs/stm32l0xx-hal/issues/98

<!-- Links to version diffs. -->
[Unreleased]: https://github.com/stm32-rs/stm32l0xx-hal/compare/v0.7.0...HEAD
[v0.7.0]: https://github.com/stm32-rs/stm32l0xx-hal/compare/v0.6.2...v0.7.0
[v0.6.2]: https://github.com/stm32-rs/stm32l0xx-hal/compare/v0.6.1...v0.6.2
[v0.6.1]: https://github.com/stm32-rs/stm32l0xx-hal/compare/v0.6.0...v0.6.1
[v0.6.0]: https://github.com/stm32-rs/stm32l0xx-hal/compare/v0.5.0...v0.6.0
