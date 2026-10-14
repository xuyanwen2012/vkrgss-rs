a, b, *c = 1, 2, 3, 4
a ||= b &&= 3
x = a <=> b
y = !(a == b) && a != b || (a === b and not b) or a
z = a ? b : c[0]
a += 1; a -= 1; a *= 2; a /= 2; a **= 2; a %= 7; a <<= 1; a >>= 1; a |= 1; a &= 3; a ^= 2
s = "a" "b" 'c'
m = -a.abs + +b - ~a
obj.attr = 1 if defined?(obj) && obj.respond_to?(:attr=)
self.class::CONST rescue ::Object
alias new_name old_name
undef gone
$g = __FILE__ + __LINE__.to_s
