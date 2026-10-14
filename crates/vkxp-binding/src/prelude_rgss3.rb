# What vkxp prepares before the scripts of a game of RGSS3 run, in the place of
# `prelude.rb`. Written for vkxp from the measurements of the original runtime; the
# functions whose names start with `vkxp_` are host functions of the binding. `rgss_main`
# and `rgss_stop` are not defined here: they come with the built-in scripts file.

# The version of RGSS that vkxp follows, as the original runtime 3.0.1 states it.
RGSS_VERSION = "3.0.1".force_encoding("ASCII-8BIT") unless defined?(RGSS_VERSION)

# Raised by Graphics.update when the host asks for a restart of the game (F12), with an
# empty message. Not a StandardError: a bare rescue does not catch it.
class RGSSReset < Exception
end

def save_data(object, filename)
  File.open(filename, "wb") { |file| file.write(Marshal.dump(object)) }
  nil
end

# `load_data` hands `Marshal.load` a proc that labels every String it is handed whose
# encoding is ASCII-8BIT as UTF-8. The encoding is asked by name: the constants of the
# encodings do not exist at start-up.
vkxp_utf8 = proc do |object|
  if object.is_a?(String) && object.encoding.name == "ASCII-8BIT"
    object.force_encoding("UTF-8")
  end
  object
end
Object.send(:define_method, :load_data) do |name|
  Marshal.load(vkxp_data_bytes(name), vkxp_utf8)
end
Object.send(:private, :load_data)

# A modal message: the text goes to the error stream and to the host, and is shown in the
# message panel until the host has dismissed it.
def vkxp_msgbox(text)
  text = text[0, text.index("\0")] if text.index("\0")
  $stderr.write(text + "\n")
  vkxp_modal(text)
  nil
end

# The arguments as `print` of Ruby 1.8 writes them: each `to_s`, `nil` as `nil`, joined
# with `$,`, and `$\` after them.
def msgbox(*args)
  text = "".force_encoding("ASCII-8BIT")
  args.each_with_index do |arg, index|
    text << $,.to_s.dup.force_encoding("ASCII-8BIT") if index > 0 && $,
    text << (arg.nil? ? "nil" : arg.to_s).dup.force_encoding("ASCII-8BIT")
  end
  text << $\.to_s.dup.force_encoding("ASCII-8BIT") if $\
  vkxp_msgbox(text)
end

def msgbox_p(*args)
  vkxp_msgbox(args.map { |arg| arg.inspect.dup.force_encoding("ASCII-8BIT") }.join("\n"))
end

# Reads the script archive as the original runtime does: each entry holds an identifier, a
# name and the deflated source, and gets the source inserted at index 3. The size is read
# once, here.
def vkxp_load_scripts(path)
  $RGSS_SCRIPTS = load_data(path)
  $RGSS_SCRIPTS.each { |script| script[3, 0] = Zlib::Inflate.inflate(script[2]) }
  $RGSS_SCRIPTS.size
end
