# The functions of Ruby 1.9 that start processes, turned off beside those of
# `prelude_process_off.rb`, which is evaluated before this file: each starts nothing,
# returns nil and says so on the error stream. Written for vkrgss.

# The rule of `vkrgss_file_name` of `prelude_process_off.rb` over Ruby 1.9: `open` gives an
# object with `to_open` to that method, and a name is converted with `to_path` before
# `to_str`.
def vkrgss_file_name(name, open)
  return name if open && name.respond_to?(:to_open)
  name = name.to_path if name.respond_to?(:to_path)
  name.respond_to?(:to_str) ? name.to_str : name
end

module Kernel
  def spawn(*args)
    vkrgss_process_function_off("spawn")
  end

  module_function :spawn
end

def Process.spawn(*args)
  vkrgss_process_function_off("Process.spawn")
end

def Process.exec(*args)
  vkrgss_process_function_off("Process.exec")
end
