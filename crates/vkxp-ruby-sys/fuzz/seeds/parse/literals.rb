a = [1, 2.5, 0x1f, 0b101, 0777, 1_000, 1e3, ?a, :sym, :"quoted sym", 'single', "dou\tble #{1 + 1} #@x #$y"]
h = { :a => 1, "b" => [nil, true, false], 3 => 1..2, 4 => (1...2) }
w = %w(one two) + %W(#{a} three)
s = %q(raw (nested)) + %Q[cooked #{h[:a]}] + `echo`
r = /ab+c/i =~ "xabbc" ? $~[0] : $1
x = <<EOS + <<-'RAW'
here #{a.size}
EOS
  raw #{not}
  RAW
