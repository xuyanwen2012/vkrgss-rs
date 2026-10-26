# The functions that start or signal processes, turned off: each starts and signals nothing,
# returns nil and says so on the error stream. `open` and the class methods of `IO` that read
# a file are turned off for a name that starts with `|` only, the name of a command; with any
# other name they work as before. Written for vkrgss.

def vkrgss_process_function_off(name, done = "started")
  $stderr.write("vkrgss: #{name} is turned off, nothing was #{done}\n")
  nil
end

# The name `open` and the class methods of `IO` take a file or a command from, converted once,
# so that the name checked is the name they get: Ruby 1.8 converts it with `to_str`.
# `prelude_process_off_ruby19.rb` replaces this with the rule of Ruby 1.9, where `open` gives
# an object with `to_open` to that method.
def vkrgss_file_name(name, open)
  name.respond_to?(:to_str) ? name.to_str : name
end

# Whether the first of `args`, which it converts by `vkrgss_file_name`, names a command.
def vkrgss_command?(args, open)
  return false if args.empty?
  args[0] = vkrgss_file_name(args[0], open)
  args[0].is_a?(String) && args[0][0, 1] == "|"
end

module Kernel
  def system(*args)
    vkrgss_process_function_off("system")
  end

  def exec(*args)
    vkrgss_process_function_off("exec")
  end

  def fork(*args)
    vkrgss_process_function_off("fork")
  end

  def syscall(*args)
    vkrgss_process_function_off("syscall")
  end

  def `(command)
    vkrgss_process_function_off("`")
  end

  # Kept once: the prelude is evaluated again in some tests of the binding.
  alias vkrgss_open open unless private_method_defined?(:vkrgss_open)

  def open(*args, &block)
    return vkrgss_process_function_off("open") if vkrgss_command?(args, true)
    vkrgss_open(*args, &block)
  end

  private :vkrgss_open
  module_function :system, :exec, :fork, :syscall, :`, :open
end

def Process.fork(*args)
  vkrgss_process_function_off("Process.fork")
end

# A module function, as Ruby has it, so that the copy a script gets with `include Process` is
# turned off too.
module Process
  def kill(*args)
    vkrgss_process_function_off("Process.kill", "signalled")
  end

  module_function :kill
end

def IO.popen(*args)
  vkrgss_process_function_off("IO.popen")
end

# `File` inherits these. `IO.binread` exists over Ruby 1.9 only.
class << IO
  unless private_method_defined?(:vkrgss_read)
    alias_method :vkrgss_read, :read
    alias_method :vkrgss_foreach, :foreach
    alias_method :vkrgss_readlines, :readlines
    private :vkrgss_read, :vkrgss_foreach, :vkrgss_readlines
  end

  def read(*args, &block)
    return vkrgss_process_function_off("IO.read") if vkrgss_command?(args, false)
    vkrgss_read(*args, &block)
  end

  def foreach(*args, &block)
    return vkrgss_process_function_off("IO.foreach") if vkrgss_command?(args, false)
    vkrgss_foreach(*args, &block)
  end

  def readlines(*args, &block)
    return vkrgss_process_function_off("IO.readlines") if vkrgss_command?(args, false)
    vkrgss_readlines(*args, &block)
  end

  if method_defined?(:binread)
    unless private_method_defined?(:vkrgss_binread)
      alias_method :vkrgss_binread, :binread
      private :vkrgss_binread
    end

    def binread(*args, &block)
      return vkrgss_process_function_off("IO.binread") if vkrgss_command?(args, false)
      vkrgss_binread(*args, &block)
    end
  end
end
