a = [1, 2.5, 1e3, 0x1f, 0b101, 0o17, 017, 1_000, ?a, ?\n, 100_000_000_000_000_000_000]
h = {:a => 1, b: 2, "c" => 3}
s = "interpolated #{a[0]} and #@x and #$y あ \x41 \101 \e"
t = 'single \' quoted'
r = /pattern #{s} (?<name>\w+)/imx
w = %w[one two three] + %W[#{s} x] rescue nil
q = %q(paren (nested)) + %Q{brace} + %(x)
sym = [:sym, :"quoted #{1}", :+, :[]=, :a?, :b!, :c=]
heredoc = <<-EOS + <<'RAW'
  text #{1 + 1}
  EOS
raw #{not}
RAW
range = (1..10).step(2).to_a + (1...3).to_a
c = `` rescue nil
str = "テスト" "連結"
