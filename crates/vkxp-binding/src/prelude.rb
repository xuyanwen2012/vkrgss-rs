# The Ruby part of what vkxp prepares before a game's scripts run. Written for vkxp; the
# functions whose names start with `vkxp_` are host functions of the binding.

$KCODE = "U"
$:.clear

# Raised by Graphics.update when the host asks for a restart of the game (F12).
class Reset < Exception
end

def save_data(object, filename)
  File.open(filename, "wb") { |file| Marshal.dump(object, file) }
end

# The text goes to the error stream and to the host, and is shown in the message panel until
# the host has dismissed it. `p` can be made log-only, which leaves the error stream.
def print(*args)
  text = ""
  args.each { |arg| text << arg.to_s }
  $stderr.write(text + "\n")
  vkxp_modal(text)
end

def p(*args)
  text = args.map { |arg| arg.inspect }.join("\n")
  $stderr.write(text + "\n")
  vkxp_modal_p(text)
end

# Reads the script archive as the original runtime does: each entry holds an identifier, a
# name and the deflated source, and gets the source as its fourth element.
def vkxp_load_scripts(path)
  $RGSS_SCRIPTS = load_data(path)
  $RGSS_SCRIPTS.each { |script| script[3] = Zlib::Inflate.inflate(script[2]) }
  $RGSS_SCRIPTS.size
end

# Calls go to the host's table of emulated functions. A function outside the table returns 0
# and is named once.
class Win32API
  @@vkxp_called = {}

  def initialize(dll, function, import, export)
    @vkxp_dll = dll.to_s
    @vkxp_function = function.to_s
    @vkxp_name = "#{dll} #{function}"
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
