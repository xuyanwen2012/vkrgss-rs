# What vkxp prepares for a game of every runtime, after `prelude.rb` or `prelude_rgss3.rb`.
# Written for vkxp; the functions whose names start with `vkxp_` are host functions of the
# binding.

# Calls go to the host's table of emulated functions. A function outside the table returns 0
# and is named once.
class Win32API
  @@vkxp_called = {}

  def initialize(dll, function, import, export)
    @vkxp_dll = dll.to_s
    @vkxp_function = function.to_s
    @vkxp_name = "#{dll} #{function}"
    vkxp_win32api_names
  end

  def call(*args)
    result = vkxp_win32api(@vkxp_dll, @vkxp_function, *args)
    return result if result
    unless @@vkxp_called[@vkxp_name]
      @@vkxp_called[@vkxp_name] = true
      $stderr.write("vkxp: Win32API #{@vkxp_name} is not emulated and returns 0\n")
    end
    0
  end

  alias Call call
end

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
