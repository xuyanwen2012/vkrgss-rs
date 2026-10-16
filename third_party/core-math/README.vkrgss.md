# CORE-MATH, vendored for vkrgss

This directory is a subset of CORE-MATH, the library of correctly rounded mathematical
functions of INRIA, kept at its original relative paths. The build scripts of
`vkrgss-ruby-sys` and `vkrgss-ruby192-sys` compile it into each interpreter, which takes the
functions of `Math` and the power of floats from it, so that a float that a script computes,
and a save that holds it, is the same on every machine and platform.

## Origin

- Repository: `https://gitlab.inria.fr/core-math/core-math.git`
- Commit: `dae5dd8c0d5bd2691b45de28c9edbe12f067c3d3` (2026-10-15)

`MANIFEST.sha256` lists the SHA-256 of every vendored file as it is at that commit, in the
format of `sha256sum`. A test in `vkrgss-ruby-sys` (`tests/manifest.rs`) checks the tree
against it: no file may be added, removed or changed without a record. A local change would be
recorded in a `PATCHES.md` with its diff under `patches/`, as for the other vendored trees;
there is none. The files that vkrgss adds (`MANIFEST.sha256`, `README.vkrgss.md` and `FILES`)
are not part of CORE-MATH.

## Licence

The MIT licence of the vkrgss repository does not cover this directory. CORE-MATH is under
the MIT licence in `LICENSE`; the copyright notice of each file is in the file.

## How the subset was chosen

- For each binary64 function that one of the interpreters uses, `src/binary64/<name>/<name>.c`:
  `acos acosh asin asinh atan atan2 atanh cbrt cos cosh erf erfc exp hypot lgamma log log10
  log2 pow sin sinh tan tanh tgamma`. Ruby 1.8.7 uses neither `cbrt`, `log2`, `tgamma` nor
  `lgamma`; Ruby 1.9.2 uses all of them.
- The headers these files include, from their own directories: `atan2/tint.h`,
  `log/dint.h`, `log10/dint.h`, `pow/pow.h`, `pow/dint.h` and `pow/qint.h`. None includes a
  header of `src/binary64/support`.
- Licence: `LICENSE`.

Everything else (the other functions and formats, each function's test programs, `.wc`
files of hard cases, MPFR references, Sollya and Sage scripts, Makefiles, documentation) is
left out. `FILES` lists the C files that the build scripts compile; both read it, so they
cannot drift apart.
