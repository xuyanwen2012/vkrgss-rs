sum = 0
[1, 2, 3].each { |n| sum += n }
[[1, 2], [3, 4]].each do |(a, b), *c|
  sum += a * b
end
square = lambda { |v| v ** 2 }
add = proc do |a, b| a + b end
def with(*args, &blk) blk.call(*args) end
with(1, 2, &add)
sum = with(3) { |v| square[v] } rescue nil
