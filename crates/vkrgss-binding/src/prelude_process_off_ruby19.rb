# The functions of Ruby 1.9 that start processes, turned off beside those of
# `prelude_process_off.rb`, which is evaluated before this file: each starts nothing,
# returns nil and says so on the error stream. `open` with a file name works as before.
# Written for vkrgss.

module Kernel
  def spawn(*args)
    vkrgss_process_function_off("spawn")
  end

  # Kept once: the prelude is evaluated again in some tests of the binding.
  alias vkrgss_open open unless private_method_defined?(:vkrgss_open)

  def open(*args, &block)
    name = args[0]
    if name.respond_to?(:to_str) && name.to_str.start_with?("|")
      return vkrgss_process_function_off("open")
    end
    vkrgss_open(*args, &block)
  end

  private :vkrgss_open
  module_function :spawn, :open
end

def Process.spawn(*args)
  vkrgss_process_function_off("Process.spawn")
end

def Process.exec(*args)
  vkrgss_process_function_off("Process.exec")
end
