sum = 0
[1, 2, 3].each { |n| sum += n }
[[1, 2], [3, 4]].each do |(a, b), *c|
  sum += a * b
end
square = ->(v) { v ** 2 }
add = proc do |a, b; local| local = a; local + b end
def with(*args, &blk) blk.call(*args) end
with(1, 2, &add)
sum = with(3) { |v| square[v] } rescue nil
yielder = lambda { |x, y = 2, *rest, &b| [x, y, rest] }
yielder.(1)
