# What vkrgss prepares before the scripts of a game of RGSS3 run, in the place of
# `prelude.rb` and after `prelude_shared.rb`. Written for vkrgss from the measurements of the
# original runtime; the functions whose names start with `vkrgss_` are host functions of the
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
  vkrgss_check_data_name(filename)
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
    vkrgss_exit_now(status)
  end

  module_function :at_exit, :exit!
end

module Process
  def exit!(status = false)
    vkrgss_exit_now(status)
  end

  module_function :exit!
end

# The rules of loading stood on the lines below. They moved to `prelude_load_rgss3.rb`,
# evaluated after this file from line 48, which a script of script mode gets too. The
# comment lines keep the line numbers of what follows.
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#

# A modal message: the text goes to the error stream and to the host, and is shown in the
# message panel until the host has dismissed it.
def vkrgss_msgbox(text)
  text = text[0, text.index("\0")] if text.index("\0")
  $stderr.write(text + "\n")
  vkrgss_modal(text)
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
  vkrgss_msgbox(text)
end

def msgbox_p(*args)
  vkrgss_msgbox(args.map { |arg| arg.inspect.dup.force_encoding("ASCII-8BIT") }.join("\n"))
end

# Reads the script archive as the original runtime does: each entry holds an identifier, a
# name and the deflated source, and gets the source inserted at index 3. The size is read
# once, here.
def vkrgss_load_scripts(path)
  $RGSS_SCRIPTS = load_data(path)
  $RGSS_SCRIPTS.each { |script| script[3, 0] = Zlib::Inflate.inflate(script[2]) }
  $RGSS_SCRIPTS.size
end
