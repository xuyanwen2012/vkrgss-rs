# The functions that start processes, turned off: each starts nothing, returns nil and says
# so on the error stream. Written for vkxp.

def vkxp_process_function_off(name)
  $stderr.write("vkxp: #{name} is turned off, nothing was started\n")
  nil
end

module Kernel
  def system(*args)
    vkxp_process_function_off("system")
  end

  def exec(*args)
    vkxp_process_function_off("exec")
  end

  def fork(*args)
    vkxp_process_function_off("fork")
  end

  def syscall(*args)
    vkxp_process_function_off("syscall")
  end

  def `(command)
    vkxp_process_function_off("`")
  end

  module_function :system, :exec, :fork, :syscall, :`
end

def Process.fork(*args)
  vkxp_process_function_off("Process.fork")
end

def IO.popen(*args)
  vkxp_process_function_off("IO.popen")
end
