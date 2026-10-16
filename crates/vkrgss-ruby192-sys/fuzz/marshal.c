/* Fuzz target: `Marshal.load` of the bytes, as for a save file or an `.rvdata2` file. */
#include "harness.h"

int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    vkrgss_fuzz_protected(rb_marshal_load, data, size, rb_ascii8bit_encoding());
    return 0;
}
