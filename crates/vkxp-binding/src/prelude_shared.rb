# What vkxp prepares for a game of every runtime, after `prelude.rb` or `prelude_rgss3.rb`.
# Written for vkxp; the functions whose names start with `vkxp_` are host functions of the
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
    path = vkxp_locate(name, true)
    return false if $".include?(path)
    $" << path
    eval(File.open(path, "rb") { |file| file.read }, TOPLEVEL_BINDING, path)
    true
  end

  def load(name, wrap = false)
    path = vkxp_locate(name, false)
    eval(File.open(path, "rb") { |file| file.read }, TOPLEVEL_BINDING, path)
    true
  end

  module_function :require, :load
end
