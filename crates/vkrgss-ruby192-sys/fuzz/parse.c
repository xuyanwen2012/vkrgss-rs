/*
 * Fuzz target: the bytes compiled as a script of source encoding UTF-8, as vkrgss compiles
 * the scripts of RGSS3 through `eval`, and not run: the instruction sequence is made and
 * dropped.
 */
#include "harness.h"
#include "vm_core.h"

static VALUE
compile(VALUE source)
{
    rb_iseq_compile(source, rb_str_new2("fuzz"), INT2FIX(1));
    return Qnil;
}

int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    /* As `eval` compiles: a syntax error becomes an exception instead of text on the error
       stream. An exception leaves the counters where they were. */
    rb_thread_t *th = GET_THREAD();
    int parse_in_eval = th->parse_in_eval;
    int mild_compile_error = th->mild_compile_error;

    th->parse_in_eval++;
    th->mild_compile_error++;
    vkrgss_fuzz_protected(compile, data, size, rb_utf8_encoding());
    th->parse_in_eval = parse_in_eval;
    th->mild_compile_error = mild_compile_error;
    return 0;
}
