# The release generates rbconfig.rb when it is built with configure and make, which vkxp does
# not run. This one holds what the test libraries and the selected tests of the release read,
# for the test executable that tests/common/mod.rs links; it is not part of the interpreter.
module RbConfig
  CONFIG = {}
  CONFIG["MAJOR"] = "1"
  CONFIG["MINOR"] = "9"
  CONFIG["TEENY"] = "2"
  CONFIG["PATCHLEVEL"] = "0"
  CONFIG["ruby_version"] = "1.9.1"
  CONFIG["arch"] = "x86_64-linux"
  CONFIG["host_os"] = "linux-gnu"
  CONFIG["target_os"] = "linux"
  CONFIG["host_cpu"] = "x86_64"
  CONFIG["target_cpu"] = "x86_64"
  CONFIG["EXEEXT"] = ""
  CONFIG["DLEXT"] = "so"
  CONFIG["ruby_install_name"] = "ruby"
  CONFIG["RUBY_INSTALL_NAME"] = "ruby"
  # The harness runs every test process in a directory that holds a link named `ruby`.
  CONFIG["bindir"] = Dir.pwd
  CONFIG["LIBPATHENV"] = "LD_LIBRARY_PATH"

  def RbConfig.ruby
    File.join(CONFIG["bindir"], CONFIG["ruby_install_name"] + CONFIG["EXEEXT"])
  end
end
Config = RbConfig unless defined?(Config)
CROSS_COMPILING = nil unless defined?(CROSS_COMPILING)
