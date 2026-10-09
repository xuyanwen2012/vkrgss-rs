# The Ruby part of what vkxp prepares before a game's scripts run. Written for vkxp; the
# functions whose names start with `vkxp_` are host functions of the binding.

$KCODE = "U"
$:.clear

# Raised by Graphics.update when the host asks for a restart of the game (F12).
class Reset < Exception
end

# Raised by the original runtime's hang watchdog, which vkxp does not have.
class Hangup < Exception
end

def save_data(object, filename)
  File.open(filename, "wb") { |file| Marshal.dump(object, file) }
end

# A placeholder: every function returns 0.
class Win32API
  @@vkxp_called = {}

  def initialize(dll, function, import, export)
    @vkxp_name = "#{dll} #{function}"
  end

  def call(*args)
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
