# encoding: utf-8
x = 3
case x
when 1, 2 then :low
when 3..5 then :mid
when Integer, /re/ then :other
else :none
end
while x > 0 do x -= 1 end
until x >= 3; x += 1 end
begin
  x += 1
end while x < 10
for i in 1..3 do next if i == 2; break if i > 2 end
loop { break }
begin
  raise ArgumentError, "bad"
rescue ArgumentError, TypeError => e
  retry if false
rescue => e
else
  x
ensure
  x = nil
end
x = if x then 1 elsif x.nil? then 2 else 3 end
unless x then 0 end
x ||= 1; x &&= 2; x += 1 if x
catch(:done) { throw :done, 1 }
__END__
data after the end
