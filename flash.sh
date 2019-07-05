#!/bin/sh
if (( $# == 1)); then
    arm-none-eabi-objcopy -O ihex "$1" program.hex
    st-flash --format ihex write program.hex
else
        echo "Usage:"
        echo "$0 <filename of firmware in ELF format>"
        exit 1
fi
