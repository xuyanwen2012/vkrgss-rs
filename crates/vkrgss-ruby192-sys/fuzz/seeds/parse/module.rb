module Vocab
  module_function
  def name; "テスト" end
end
module Cache
  @cache = {}
  def self.load(path, hue = 0)
    @cache[[path, hue]] ||= path.dup
  end
end
class Game < Struct.new(:a, :b)
  include Comparable
  extend Enumerable
  def each; yield a; yield b end
end
undef_method rescue nil
::Vocab::name
defined?(Cache) && defined?(@x).nil?
