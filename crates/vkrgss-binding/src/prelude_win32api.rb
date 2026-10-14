# `Win32API` for a game of RGSS1 or RGSS2, evaluated before `prelude_shared.rb`. Written for
# vkrgss; the functions whose names start with `vkrgss_` are host functions of the binding. In
# RGSS3 the class is one of the binding (`win32api.rs`).

# Calls go to the host's table of emulated functions. A function outside the table returns 0
# and is named once.
class Win32API
  @@vkrgss_called = {}

  def initialize(dll, function, import, export)
    @vkrgss_dll = dll.to_s
    @vkrgss_function = function.to_s
    @vkrgss_name = "#{dll} #{function}"
    vkrgss_win32api_names
  end

  def call(*args)
    result = vkrgss_win32api(@vkrgss_dll, @vkrgss_function, *args)
    return result if result
    unless @@vkrgss_called[@vkrgss_name]
      @@vkrgss_called[@vkrgss_name] = true
      $stderr.write("vkrgss: Win32API #{@vkrgss_name} is not emulated and returns 0\n")
    end
    0
  end

  alias Call call
end
