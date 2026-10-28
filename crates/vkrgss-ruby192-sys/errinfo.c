/* Sets the current error of the running thread as the interpreter itself puts it back after
 * a tag jump (`th->errinfo = errinfo` in rb_ensure() and in the VM), without the check of
 * rb_set_errinfo(), which raises TypeError for a value that is not an exception, such as
 * what a pending `throw` carries. The protected call of vkrgss-ruby-glue puts the previous
 * error back with it when that error is neither nil nor an exception. */
#include "ruby/ruby.h"
#include "vm_core.h"

void ruby_vkxp_set_errinfo(VALUE errinfo);

void
ruby_vkxp_set_errinfo(VALUE errinfo)
{
    GET_THREAD()->errinfo = errinfo;
}
