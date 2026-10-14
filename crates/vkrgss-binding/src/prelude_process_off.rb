# The functions that start processes, turned off: each starts nothing, returns nil and says
# so on the error stream. Written for vkrgss.

def vkrgss_process_function_off(name)
  $stderr.write("vkrgss: #{name} is turned off, nothing was started\n")
  nil
end

module Kernel
  def system(*args)
    vkrgss_process_function_off("system")
  end

  def exec(*args)
    vkrgss_process_function_off("exec")
  end

  def fork(*args)
    vkrgss_process_function_off("fork")
  end

  def syscall(*args)
    vkrgss_process_function_off("syscall")
  end

  def `(command)
    vkrgss_process_function_off("`")
  end

  module_function :system, :exec, :fork, :syscall, :`
end

def Process.fork(*args)
  vkrgss_process_function_off("Process.fork")
end

def IO.popen(*args)
  vkrgss_process_function_off("IO.popen")
end
