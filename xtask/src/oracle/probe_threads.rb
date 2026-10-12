
# Added with `--no-threads`: a Ruby thread ends the original runtime under Wine. The block of
# `Thread.new` runs at once in the caller, and `sleep` returns at once inside it.
class Thread
  def self.new(*arguments)
    outer = $oracle_probe_in_thread
    $oracle_probe_in_thread = true
    begin
      yield(*arguments)
    ensure
      $oracle_probe_in_thread = outer
    end
    Thread.current
  end
end

module Kernel
  alias_method :oracle_probe_sleep, :sleep

  def sleep(*arguments)
    $oracle_probe_in_thread ? 0 : oracle_probe_sleep(*arguments)
  end
end
