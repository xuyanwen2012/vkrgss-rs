module Outer
  CONST = 1
  class Inner < ::Object
    include Comparable
    class << self
      def make; new; end
    end
    def <=>(other) 0 end
    def [](i) i end
    def []=(i, v) v end
    def +@; self end
    def name?() true end
    def name!() end
    private
    def self.build(a, b = a) [a, b] end
  end
end
BEGIN { 1 }
END { 2 }
=begin
a comment block
=end
__END__
not code
