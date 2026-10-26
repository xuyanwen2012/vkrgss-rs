# The rules of loading of RGSS3, after `prelude_rgss3.rb` and evaluated from line 48, where
# they stood in it; a script of script mode over Ruby 1.9 gets them after `prelude_shared.rb`.
# Only Ruby files inside the game directory are loaded, and `require` and `load` replace
# those of `prelude_shared.rb`: a file is compiled as UTF-8, as the scripts are.
# `require_relative` takes the name relative to the directory of the file it is called from,
# and `autoload` locates the file by the rule of `require` at the call.
module Kernel
  def require(name)
    path = vkrgss_locate(name, true)
    vkrgss_require(path) do
      eval(File.open(path, "rb") { |file| file.read }.force_encoding("UTF-8"),
           TOPLEVEL_BINDING, path)
    end
  end

  # `vkrgss_require` and `vkrgss_load` are those of `prelude_shared.rb`.
  def load(name, wrap = false)
    path = vkrgss_locate(name, false)
    vkrgss_load(File.open(path, "rb") { |file| file.read }.force_encoding("UTF-8"),
                path, wrap)
    true
  end

  def require_relative(name)
    file = caller(1)[0].to_s[/\A(.*?):\d+(?::in |\z)/, 1]
    require(vkrgss_relative(file.to_s, name))
  end

  # Registered in `Object`, the namespace of a call at the top level.
  def autoload(name, file)
    Object.autoload(name, file)
  end

  module_function :require, :load, :require_relative, :autoload
end

# Ruby's own `autoload` loads the file by its path at the first use of the constant.
class Module
  unless private_method_defined?(:vkrgss_autoload)
    alias_method :vkrgss_autoload, :autoload
    private :vkrgss_autoload
  end

  def autoload(name, file)
    vkrgss_autoload(name, vkrgss_locate(file, true))
  end
end
