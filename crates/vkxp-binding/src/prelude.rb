# What vkxp prepares before the scripts of a game of RGSS1 or RGSS2 run. Written for vkxp;
# the functions whose names start with `vkxp_` are host functions of the binding.

# `$KCODE` and the load path are set over Ruby 1.8 only: `prelude_ruby18.rb`, which is
# evaluated before this file.

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

# `Win32API`, `require` and `load` are what every runtime gets: `prelude_shared.rb`, which is
# evaluated after this file.
