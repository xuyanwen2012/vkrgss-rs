/*
 * Fuzz target: the bytes up to the first line break compiled as a pattern in UTF-8, as
 * `Regexp.new` compiles a string of a script of RGSS3, and the rest, also in UTF-8, matched
 * against it.
 */
#include <string.h>

#include "harness.h"
#include "ruby/re.h"

static VALUE
compile_and_match(VALUE input)
{
    rb_encoding *utf8 = rb_utf8_encoding();
    const char *bytes = RSTRING_PTR(input);
    long size = RSTRING_LEN(input);
    const char *line_break = memchr(bytes, '\n', (size_t)size);
    long pattern_size = line_break ? line_break - bytes : size;
    long subject_at = line_break ? pattern_size + 1 : size;
    volatile VALUE subject = rb_enc_str_new(bytes + subject_at, size - subject_at, utf8);
    volatile VALUE pattern = rb_enc_reg_new(bytes, pattern_size, utf8, 0);

    rb_reg_search(pattern, subject, 0, 0);
    return Qnil;
}

int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    vkrgss_fuzz_protected(compile_and_match, data, size, rb_utf8_encoding());
    return 0;
}
