i = 0
while i < 10
  i += 1
  next if i % 2 == 0
  break unless i < 8
end
for x in 1..3 do puts x end
until i.zero? ; i -= 1 ; redo if false ; end
case i
when 0, 1 then :small
when Integer then :number
else :other
end
begin
  raise ArgumentError, "no" if i > 0
rescue ArgumentError, TypeError => e
  retry if false
else
  1
ensure
  2
end
