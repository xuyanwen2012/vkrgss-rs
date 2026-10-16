a, b, *c = 1, 2, 3, 4
d = a + b - c.size * 2 / 1 % 3 ** 2
e = a & b | d ^ ~a << 2 >> 1
f = !a && b || not(c) and d or e
g = a <=> b; g = a == b; g = a != b; g = a =~ /x/; g = a !~ /y/
h = a ? b : c
i = -a + +b
j = a.b rescue a::b rescue nil
k = [*c, *[1]]
(n, o), p = [1, 2], 3
q = defined? a
r = a.instance_eval { self } .then rescue nil
s = 1.+(2).-(3)
t ||= u = 5
v = a < b && b <= c.size || a > b && a >= 0
w = begin 1; 2 end.to_s.length
