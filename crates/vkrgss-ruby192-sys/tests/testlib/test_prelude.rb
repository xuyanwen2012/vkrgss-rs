# Loaded with -r before test/runner.rb. The test executable has the release's miniprelude
# and not its gem prelude; test/ruby/envutil.rb reads Gem::ConfigMap, which the gem prelude
# would define.
require 'rbconfig'
module Gem
  ConfigMap = {} unless defined?(ConfigMap)
  ConfigMap[:bindir] = RbConfig::CONFIG["bindir"]
  ConfigMap[:ruby_install_name] = RbConfig::CONFIG["ruby_install_name"]
  ConfigMap[:EXEEXT] = RbConfig::CONFIG["EXEEXT"]
end

# The suites name encodings by their constants (Encoding::ISO_8859_1). vkrgss registers
# Shift_JIS, Windows-31J and ISO-8859-1 at the first look-up of their name, and an encoding
# has no constant before it is registered.
%w[Shift_JIS Windows-31J ISO-8859-1].each {|name| Encoding.find(name) }
