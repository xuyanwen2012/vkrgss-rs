# Test vectors of the correctly rounded functions

`vectors.txt` holds 5851 lines `name,input[,input2],result`, every number a C99 hexadecimal
float: for each of the 24 functions that the interpreters take from CORE-MATH
(`third_party/core-math`), a sample of the hard cases of CORE-MATH's `.wc` files and random
inputs, each with its exact result rounded to nearest. For `atan2` the first input is `y`;
`lgamma` is the logarithm of the absolute value of the gamma function. The tests `math.rs`
of `vkrgss-ruby-sys` and `vkrgss-ruby192-sys` evaluate each line of a function that their
interpreter has through `Math` or `**` and compare the bits of the result.

`gen.py` wrote the file on 2026-10-16, from the `.wc` files of CORE-MATH at commit
`dae5dd8c0d5bd2691b45de28c9edbe12f067c3d3`, with mpmath 1.3.0 at 400 bits and the seed
20261016. Regenerating it needs mpmath and a clone of CORE-MATH at that commit:

    python3 gen.py <core-math>/src/binary64

writes `vectors.txt` to the current directory.
