# What vkrgss prepares for a game of every runtime, after `prelude.rb` or `prelude_rgss3.rb`.
# Written for vkrgss; `vkrgss_locate` is a host function of the binding, and the other functions
# whose names start with `vkrgss_` are defined below.

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
    vkrgss_require(path) do
      eval(File.open(path, "rb") { |file| file.read }, TOPLEVEL_BINDING, path)
    end
  end

  # With `wrap` the file is evaluated in an anonymous module.
  def load(name, wrap = false)
    path = vkrgss_locate(name, false)
    vkrgss_load(File.open(path, "rb") { |file| file.read }, path, wrap)
    true
  end

  module_function :require, :load
end

# What `require` does with the located `path`, here and in `prelude_load_rgss3.rb`, as the
# interpreters' own `require` does it: a file is recorded in `$"` once it has loaded, so that
# a file that raised can be required again, and is not loaded again while it loads, which
# `$vkrgss_loading` tells. The block loads the file.
$vkrgss_loading = []

def vkrgss_require(path)
  return false if $".include?(path) || $vkrgss_loading.include?(path)
  $vkrgss_loading << path
  begin
    yield
  ensure
    $vkrgss_loading.delete(path)
  end
  $" << path
  true
end

# Evaluates `source` of the file `path` at the top level, or with `wrap` in an anonymous
# module, which keeps the constants and methods the file defines.
def vkrgss_load(source, path, wrap)
  if wrap
    Module.new.module_eval(source, path)
  else
    eval(source, TOPLEVEL_BINDING, path)
  end
end

# `autoload` locates the file by the rule of `require` at the call and registers the located
# path, which the interpreter's own loader loads at the first use of the constant.
# `prelude_load_rgss3.rb` defines the same for RGSS3, after this file.
module Kernel
  # Registered in `Object`, the namespace of a call at the top level.
  def autoload(name, file)
    Object.autoload(name, file)
  end

  module_function :autoload
end

class Module
  unless private_method_defined?(:vkrgss_autoload)
    alias_method :vkrgss_autoload, :autoload
    private :vkrgss_autoload
  end

  def autoload(name, file)
    vkrgss_autoload(name, vkrgss_locate(file, true))
  end
end
