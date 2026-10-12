/* Fuzz target: the bytes compiled as a script, as `eval` compiles a string, and not run. */
#include "harness.h"

static VALUE
compile(VALUE source)
{
    rb_compile_string("fuzz", source, 1);
    return Qnil;
}

int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    /* Inside `eval` a syntax error becomes an exception instead of text on the error
       stream. An exception leaves the counter where it was. */
    ruby_in_eval = 1;
    vkxp_fuzz_protected(compile, data, size);
    ruby_in_eval = 0;
    ruby_nerrs = 0;
    return 0;
}
