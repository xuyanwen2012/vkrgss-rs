class Window_Example < Object
  attr_accessor :x, :y
  CONSTANT = 10
  @@count = 0
  def initialize(x = 0, y = 0)
    super()
    @x, @y = x, y
    @@count += 1
  end
  def <=>(other) x <=> other.x end
  def [](index) index.zero? ? @x : @y end
  def []=(index, value) index.zero? ? (@x = value) : (@y = value) end
  def self.count; @@count end
  class << self
    def build(*a) new(*a) end
  end
  protected
  def hidden; end
  private :hidden
  alias_method :position, :x
  alias size position
end
