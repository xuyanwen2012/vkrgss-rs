/* Fuzz target: `Marshal.load` of the bytes, as for a save file or an `.rxdata` file. */
#include "harness.h"

int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    vkrgss_fuzz_protected(rb_marshal_load, data, size);
    return 0;
}
