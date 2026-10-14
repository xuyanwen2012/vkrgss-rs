# What vkxp prepares before the scripts of a game of RGSS3 run, in the place of
# `prelude.rb` and after `prelude_shared.rb`. Written for vkxp from the measurements of the
# original runtime; the functions whose names start with `vkxp_` are host functions of the
# binding, and so is `load_data`. `rgss_main` and `rgss_stop` are not defined here: they
# come with the built-in scripts file.

# RGSS_VERSION is defined by the binding before this file, by the version of the DLL the
# game ships: "3.0.0" for 3.0.0.x and "3.0.1" otherwise.

# Raised by Graphics.update when the host asks for a restart of the game (F12), with an
# empty message. Not a StandardError: a bare rescue does not catch it.
class RGSSReset < Exception
end

# The name is checked as `load_data` checks it before anything is written. `load_data` is
# a host function of the binding.
def save_data(object, filename)
  vkxp_check_data_name(filename)
  File.open(filename, "wb") { |file| file.write(Marshal.dump(object)) }
  nil
end

# The original runtime never runs an `at_exit` block: `at_exit` returns the block and
# registers nothing. `exit!` and `Process.exit!` end the game at once, without running an
# `ensure` clause, through the host function, which sends the outcome and the shutdown
# message first.
module Kernel
  def at_exit(&block)
    raise ArgumentError, "called without a block" unless block
    block
  end

  def exit!(status = false)
    vkxp_exit_now(status)
  end

  module_function :at_exit, :exit!
end

module Process
  def exit!(status = false)
    vkxp_exit_now(status)
  end

  module_function :exit!
end

# Only Ruby files inside the game directory are loaded, by the rule of `prelude_shared.rb`,
# which is evaluated before this file in a game of RGSS3 and whose `require` and `load`
# these replace: a file is compiled as UTF-8, as the scripts are. `require_relative` takes
# the name relative to the directory of the file it is called from, the game directory
# for a script, and `autoload` locates the file by the rule of `require` at the call;
# Ruby's own would call the interpreter's `require` past that rule.
module Kernel
  def require(name)
    path = vkxp_locate(name, true)
    return false if $".include?(path)
    $" << path
    eval(File.open(path, "rb") { |file| file.read }.force_encoding("UTF-8"),
         TOPLEVEL_BINDING, path)
    true
  end

  def load(name, wrap = false)
    path = vkxp_locate(name, false)
    eval(File.open(path, "rb") { |file| file.read }.force_encoding("UTF-8"),
         TOPLEVEL_BINDING, path)
    true
  end

  def require_relative(name)
    file = caller(1)[0].to_s[/\A(.*?):\d+(?::in |\z)/, 1]
    require(vkxp_relative(file.to_s, name))
  end

  # Registered in `Object`, the namespace of a call at the top level.
  def autoload(name, file)
    Object.autoload(name, file)
  end

  module_function :require, :load, :require_relative, :autoload
end

# Ruby's own `autoload` loads the file by its path at the first use of the constant.
class Module
  unless private_method_defined?(:vkxp_autoload)
    alias_method :vkxp_autoload, :autoload
    private :vkxp_autoload
  end

  def autoload(name, file)
    vkxp_autoload(name, vkxp_locate(file, true))
  end
end

# A modal message: the text goes to the error stream and to the host, and is shown in the
# message panel until the host has dismissed it.
def vkxp_msgbox(text)
  text = text[0, text.index("\0")] if text.index("\0")
  $stderr.write(text + "\n")
  vkxp_modal(text)
  nil
end

# The arguments as `print` of Ruby 1.8 writes them: each `to_s`, `nil` as `nil`, joined
# with `$,`, and `$\` after them.
def msgbox(*args)
  text = "".force_encoding("ASCII-8BIT")
  args.each_with_index do |arg, index|
    text << $,.to_s.dup.force_encoding("ASCII-8BIT") if index > 0 && $,
    text << (arg.nil? ? "nil" : arg.to_s).dup.force_encoding("ASCII-8BIT")
  end
  text << $\.to_s.dup.force_encoding("ASCII-8BIT") if $\
  vkxp_msgbox(text)
end

def msgbox_p(*args)
  vkxp_msgbox(args.map { |arg| arg.inspect.dup.force_encoding("ASCII-8BIT") }.join("\n"))
end

# Reads the script archive as the original runtime does: each entry holds an identifier, a
# name and the deflated source, and gets the source inserted at index 3. The size is read
# once, here.
def vkxp_load_scripts(path)
  $RGSS_SCRIPTS = load_data(path)
  $RGSS_SCRIPTS.each { |script| script[3, 0] = Zlib::Inflate.inflate(script[2]) }
  $RGSS_SCRIPTS.size
end
