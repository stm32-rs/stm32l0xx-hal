#include <stdint.h>


// Called from src/flash.rs. See comment there for an explanation of why a C
// function is necessary.
__attribute__((section(".data")))
void write_half_page(uint32_t *address, uint32_t *words) {
    uint32_t i = 0;
    while (i < 16) {
        *(address + i) = *(words + i);
        i++;
    }
}
