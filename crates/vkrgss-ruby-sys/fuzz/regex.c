/*
 * Fuzz target: the bytes up to the first line break compiled as a pattern, as `Regexp.new`
 * compiles a string, and the rest matched against it.
 */
#include <string.h>

#include "harness.h"

static VALUE
compile_and_match(VALUE input)
{
    const char *bytes = RSTRING(input)->ptr;
    long size = RSTRING(input)->len;
    const char *line_break = memchr(bytes, '\n', (size_t)size);
    long pattern_size = line_break ? line_break - bytes : size;
    long subject_at = line_break ? pattern_size + 1 : size;
    volatile VALUE subject = rb_str_new(bytes + subject_at, size - subject_at);
    volatile VALUE pattern = rb_reg_new(bytes, pattern_size, 0);

    rb_reg_search(pattern, subject, 0, 0);
    return Qnil;
}

int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    vkrgss_fuzz_protected(compile_and_match, data, size);
    return 0;
}
