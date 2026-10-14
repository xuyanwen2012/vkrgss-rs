# `DL` for a game of RGSS3, evaluated after `prelude_shared.rb`. Written for vkrgss from the
# method list of the original runtime of RGSS3: the module with its numeric constants and
# its classes. vkrgss loads no library and reads no memory of the process, so every function
# of the module and every constructor and class method of its classes raises `DL::DLError`,
# which is what a script rescues for a library that cannot be opened.

module DL
  TYPE_VOID = 0
  TYPE_VOIDP = 1
  TYPE_CHAR = 2
  TYPE_SHORT = 3
  TYPE_INT = 4
  TYPE_LONG = 5
  TYPE_LONG_LONG = 6
  TYPE_FLOAT = 7
  TYPE_DOUBLE = 8

  SIZEOF_CHAR = 1
  SIZEOF_SHORT = 2
  SIZEOF_INT = 4
  SIZEOF_LONG = 4
  SIZEOF_LONG_LONG = 8
  SIZEOF_FLOAT = 4
  SIZEOF_DOUBLE = 8
  SIZEOF_VOIDP = 4

  ALIGN_CHAR = 1
  ALIGN_SHORT = 2
  ALIGN_INT = 4
  ALIGN_LONG = 4
  ALIGN_LONG_LONG = 8
  ALIGN_FLOAT = 4
  ALIGN_DOUBLE = 8
  ALIGN_VOIDP = 4

  RTLD_GLOBAL = -1
  RTLD_LAZY = -1
  RTLD_NOW = -1

  DLSTACK_SIZE = 20
  MAX_CALLBACK = 5

  class DLError < StandardError
  end

  class DLTypeError < DLError
  end

  # Raises `DLError` for the call `name`.
  def self.vkrgss_refuse(name)
    raise DLError, "#{name}: vkrgss loads no library and reads no memory of the process"
  end

  def dlopen(*args)
    DL.send(:vkrgss_refuse, "DL.dlopen")
  end

  def dlunwrap(address)
    DL.send(:vkrgss_refuse, "DL.dlunwrap")
  end

  def dlwrap(object)
    DL.send(:vkrgss_refuse, "DL.dlwrap")
  end

  def free(address)
    DL.send(:vkrgss_refuse, "DL.free")
  end

  def malloc(size)
    DL.send(:vkrgss_refuse, "DL.malloc")
  end

  def realloc(address, size)
    DL.send(:vkrgss_refuse, "DL.realloc")
  end

  module_function :dlopen, :dlunwrap, :dlwrap, :free, :malloc, :realloc

  class << self
    private :vkrgss_refuse
  end

  class Handle
    def self.[](name)
      DL.send(:vkrgss_refuse, "DL::Handle[]")
    end

    def self.sym(name)
      DL.send(:vkrgss_refuse, "DL::Handle.sym")
    end

    def initialize(*args)
      DL.send(:vkrgss_refuse, "DL::Handle.new")
    end
  end

  class CFunc
    def initialize(*args)
      DL.send(:vkrgss_refuse, "DL::CFunc.new")
    end
  end

  class CPtr
    def self.[](value)
      DL.send(:vkrgss_refuse, "DL::CPtr[]")
    end

    def self.malloc(*args)
      DL.send(:vkrgss_refuse, "DL::CPtr.malloc")
    end

    def self.to_ptr(value)
      DL.send(:vkrgss_refuse, "DL::CPtr.to_ptr")
    end

    def initialize(*args)
      DL.send(:vkrgss_refuse, "DL::CPtr.new")
    end
  end
end
