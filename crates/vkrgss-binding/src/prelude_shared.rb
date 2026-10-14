# What vkrgss prepares for a game of every runtime, after `prelude.rb` or `prelude_rgss3.rb`.
# Written for vkrgss; the functions whose names start with `vkrgss_` are host functions of the
# binding.

# `Win32API` was defined on the lines below. It moved to `prelude_win32api.rb`, evaluated
# before this file for RGSS1 and RGSS2; in RGSS3 it is a class of the binding. The comment
# lines keep the line numbers of what follows.
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

# Only Ruby files inside the game directory are loaded.
module Kernel
  def require(name)
    path = vkrgss_locate(name, true)
    return false if $".include?(path)
    $" << path
    eval(File.open(path, "rb") { |file| file.read }, TOPLEVEL_BINDING, path)
    true
  end

  def load(name, wrap = false)
    path = vkrgss_locate(name, false)
    eval(File.open(path, "rb") { |file| file.read }, TOPLEVEL_BINDING, path)
    true
  end

  module_function :require, :load
end
