# Design

## Context

Upstream fix (ruby/ruby 33e5b47c, in Onigmo's `regexec.c`):

```c
 backward_search_range(regex_t* reg, const UChar* str, const UChar* end,
 		      UChar* s, const UChar* range, UChar* adjrange,
 		      UChar** low, UChar** high)
 {
   UChar *p;
+  size_t input_len = end - str;
+
+  if (reg->dmin > input_len) {
+    return 0;
+  }

   range += reg->dmin;
```

In both trees, `backward_search_range` starts with `range += reg->dmin; p = s;` and has no
such check. The return convention of the function in each tree must be read before the
patch is written: "not found" is whatever value the callers treat as no match (in Onigmo
`0`; check `onig_search` of each tree).

## Decisions

- One patch per tree, named like the others of its series (for example
  `engine-backward-search-dmin.patch` in Ruby 1.9.2 and the next free name in Oniguruma
  5.9.6), with an entry in `PATCHES.md` that names 33e5b47c and the row of
  `docs/vendored-security.md`, and the checksums the manifest test checks.
- The type of the comparison follows the tree: `reg->dmin` is `OnigDistance` there; compare
  it with the length as an unsigned value of the same width, without a cast that could
  wrap.
- Tests: one in each interpreter crate, in the file where that crate keeps its backports
  (`tests/backports.rs` in `vkrgss-ruby192-sys`; the equivalent in `vkrgss-ruby-sys`). Each
  runs backward searches whose least length exceeds the subject: `"ab".rindex(/abc/)`, a
  pattern with a literal prefix, one with a character class and one with an anchor, so that
  the optimizations the backward search takes (exact, exact with Boyer-Moore where the
  backward search uses it, map) are all reached, and checks they return `nil`. If a
  sanitized build shows a report for one of them before the patch, record that in the
  test's comment; if none does, say so in `design.md`.
- `docs/vendored-security.md`: the row of CVE-2024-27282 in the section of 1.9.2 says the
  backward check is backported, and the rows of CVE-2017-9227 (both sections) mention it.

## Out of scope

Other parts of 33e5b47c, if any, beyond the two checks; any other row of the review.

## Follow-ups

None yet.
