class Counter
  attr_reader :count
  def initialize(start = 0)
    @count = start
  end
  def step(by = 1, *rest, &block)
    @count += by
    yield @count if block_given?
    self
  end
end
