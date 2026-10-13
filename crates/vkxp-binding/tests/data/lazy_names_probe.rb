# The probe of first uses: the only script of a small game of RGSS2. It was written for vkxp
# and run on the original runtime of VX (RGSS202E 2.0.2.1, 2026-10-12) to find the names the
# runtime interns at the first use of a function, after the game's scripts have started;
# `lazy_names_probe.original.txt` holds what it measured there, and the test
# `run_game_lazy_names.rs` runs it in vkxp.
#
# The whole script is parsed before it runs, so every identifier written here has its number
# before the first step. Each step is a block; after it the probe notes the number of a
# marker symbol made with String#to_sym, which shows how far the sequence of symbol numbers
# has moved, and the symbols that appeared during the step, which can only come from the
# runtime's own code. Output: lazy.txt in the game directory, one line per step:
#   index|label|ok or the exception class|marker number >> 3|number name, number name ...
$zz_known = {}
$zz_out = []
$zz_steps = []
def zz_scan
  zz_new = []
  Symbol.all_symbols.each do |zz_s|
    zz_i = zz_s.to_i
    next if $zz_known[zz_i]
    $zz_known[zz_i] = true
    zz_new.push([zz_i, zz_s.to_s])
  end
  zz_new
end
def zz_step(zz_label, &zz_block)
  $zz_steps.push([zz_label, zz_block])
end
def zz_run
  zz_scan
  zz_index = 0
  $zz_steps.each do |zz_pair|
    zz_status = "ok"
    begin
      zz_pair[1].call
    rescue Exception => zz_error
      zz_status = zz_error.class.to_s
    end
    zz_new = zz_scan
    zz_marker = ("zz_m" + zz_index.to_s).to_sym.to_i
    $zz_known[zz_marker] = true
    zz_names = zz_new.sort.map { |zz_n| zz_n[0].to_s + " " + zz_n[1] }
    $zz_out.push([zz_index.to_s, zz_pair[0], zz_status, (zz_marker >> 3).to_s, zz_names.join(", ")].join("|"))
    zz_index += 1
    File.open("lazy.txt", "wb") { |zz_file| zz_file.write($zz_out.join("\n") + "\n") }
  end
end

class ZZ_Thing
  attr_accessor :zz_attr
  def initialize
    @zz_attr = 1
    @zz_other = [1, 2.5, "s", :zz_symbol, nil, true]
  end
end
ZZ_Struct = Struct.new(:zz_one, :zz_two)

# --- calibration and the interpreter itself
zz_step("nothing") { }
zz_step("nothing again") { }
zz_step("raise and rescue") { raise "zz" }
zz_step("raise NoMethodError") { nil.zz_missing }
zz_step("raise NameError") { zz_undefined_name }
zz_step("raise ZeroDivisionError") { 1 / 0 }
zz_step("raise TypeError") { 1 + nil }
zz_step("raise ArgumentError") { Integer("zz") }
zz_step("raise Errno::ENOENT") { File.open("zz-no-such-file", "rb") { } }
zz_step("catch throw") { catch(:zz_symbol) { throw :zz_symbol } }
zz_step("string ops") { "a,b".split(","); "abc".sub(/b/) { "x" }; "abc".gsub("b", "c"); "abc" =~ /b/; "%05.2f %s %d" % [1.5, "s", 3]; "abc".unpack("U*").pack("U*"); "a".succ; "abc".scan(/./); "abc".tr("a", "b"); "1".to_i; "1.5".to_f; "a".to_sym; "abc"[1, 1]; "abc".inspect; "abc".hash; "b" <=> "a"; "abc".upcase; "a" * 3; "abc".reverse; "abc".index("b"); "abc".each_byte { }; "a\nb".each_line { }; "abc".strip; "abc".center(9) }
zz_step("array ops") { [3, 1, 2].sort; [3, 1, 2].sort { |zz_a, zz_b| zz_b <=> zz_a }; [1, 2].inspect; [1, [2]].flatten; [1, 2].join(","); [1, 2].include?(2); [1, 2].map { |zz_v| zz_v }; [1, 2].inject(0) { |zz_a, zz_b| zz_a + zz_b }; [1, 2].sort_by { |zz_v| -zz_v }; [1, 1].uniq; [1, nil].compact; [1, 2].pack("CC"); [1, 2].max; [1, 2].min; [[1, 2]].transpose; [1, 2] <=> [1, 3]; [1, 2] == [1, 2]; [1, 2].hash; [1, 2].each_with_index { }; [1, 2].zip([3, 4]); [1, 2].find { |zz_v| zz_v == 2 }; [1, 2].select { |zz_v| zz_v }; [1, 2].delete_if { |zz_v| zz_v }; [1, 2].reverse.first; [1, 2].dup; [1, 2].clone; Array.new(3) { |zz_v| zz_v } }
zz_step("hash ops") { zz_h = { 1 => 2, "a" => 3, :zz_symbol => 4, 1.5 => 5, [1] => 6 }; zz_h.inspect; zz_h.keys; zz_h.each { |zz_k, zz_v| }; zz_h.sort_by { |zz_k, zz_v| zz_v }; zz_h.dup; zz_h.delete(1); zz_h.to_a; zz_h == zz_h.clone; Hash.new { |zz_a, zz_b| zz_a[zz_b] = 1 }[2]; zz_h.merge(zz_h); zz_h.invert; zz_h.index(4); zz_h.select { |zz_k, zz_v| true }; zz_h.find { |zz_k, zz_v| true }; zz_h.rehash }
zz_step("numeric ops") { 1.5.to_s; 1.5.round; 2 ** 70; (2 ** 70).to_s; 7.divmod(2); 1.5.inspect; Math.sin(1.0); Math.sqrt(2); rand(5); rand; srand(1); 1.0 / 3; 10.times { }; 1.upto(2) { }; 1.step(5, 2) { }; (1..3).each { }; (1..3).to_a; (1...3).include?(2); 3.zero?; 3 <=> 4; 3.0 == 3; 1.coerce(2.5); Integer("12"); Float("1.5"); 255.chr; 3.to_f; 1.5.floor; 1.5.ceil; -1.abs; [1.5, 2].max; 5.between?(1, 9); 1.5.nan? }
zz_step("object ops") { zz_o = ZZ_Thing.new; zz_o.inspect; zz_o.to_s; zz_o.dup; zz_o.clone; zz_o.zz_attr = 2; zz_o.respond_to?(:zz_attr); zz_o.send(:zz_attr); zz_o.method(:zz_attr).call; zz_o.instance_variables; zz_o.instance_variable_get("@zz_attr"); zz_o.instance_variable_set("@zz_attr", 3); zz_o.instance_eval { @zz_attr }; zz_o.is_a?(ZZ_Thing); zz_o.kind_of?(Object); zz_o == zz_o; zz_o.equal?(zz_o); zz_o.hash; zz_o.object_id; zz_o.freeze; zz_o.frozen?; zz_o.methods; zz_o.class.name; zz_o.class.instance_methods(false); ZZ_Thing.ancestors; zz_o.nil?; zz_o.instance_of?(ZZ_Thing); zz_o.singleton_methods }
zz_step("struct ops") { zz_o = ZZ_Struct.new(1, 2); zz_o.zz_one; zz_o.zz_two = 3; zz_o.inspect; zz_o.to_a; zz_o == zz_o.dup; zz_o[0]; zz_o.members; Marshal.load(Marshal.dump(zz_o)) }
zz_step("module ops") { zz_c = Class.new(ZZ_Thing); zz_c.class_eval { def zz_attr; 5; end }; zz_c.new.zz_attr; zz_c.module_eval("7"); eval("8"); ZZ_Thing.const_defined?(:ZZ_Struct); Object.const_get(:ZZ_Struct); ZZ_Thing.method_defined?(:zz_attr); ZZ_Thing.instance_method(:zz_attr); Comparable.instance_methods; ZZ_Thing.class_eval { alias_method :zz_scan, :zz_attr; public :zz_scan; private :zz_scan }; binding; block_given?; caller; proc { |zz_a| zz_a }.call(1); lambda { 1 }.arity; method(:zz_scan).to_proc; ObjectSpace.garbage_collect; GC.start; defined?(zz_c); __FILE__; $!; $@; $~; $0 }
zz_step("marshal plain") { Marshal.load(Marshal.dump([1, "s", :zz_symbol, 1.5, nil, true, { 1 => 2 }, 2 ** 70, ZZ_Thing.new, 1..2, ZZ_Thing, Comparable, "a" * 300])) }
zz_step("marshal to file") { File.open("zz-m.bin", "wb") { |zz_f| Marshal.dump(ZZ_Thing.new, zz_f) }; File.open("zz-m.bin", "rb") { |zz_f| Marshal.load(zz_f) } }
zz_step("file ops") { File.open("zz-f.txt", "w") { |zz_f| zz_f.write("a\nb\n"); zz_f.puts("c"); zz_f.print("d"); zz_f << "e" }; File.open("zz-f.txt", "r") { |zz_f| zz_f.gets; zz_f.read; zz_f.eof?; zz_f.rewind; zz_f.readlines; zz_f.pos }; FileTest.exist?("zz-f.txt"); File.exist?("zz-f.txt"); File.size("zz-f.txt"); File.mtime("zz-f.txt"); File.stat("zz-f.txt").size; File.basename("a/b.c", ".c"); File.dirname("a/b"); File.extname("a.b"); File.join("a", "b"); File.expand_path("zz-f.txt"); Dir.pwd; Dir.entries("."); Dir.glob("*.txt"); Dir["*.ini"]; File.delete("zz-f.txt"); IO.readlines("Game.ini"); File.file?("Game.ini"); File.directory?("Data"); File.rename("zz-m.bin", "zz-n.bin"); Dir.mkdir("zz-dir"); Dir.rmdir("zz-dir") }
zz_step("time ops") { zz_t = Time.now; zz_t.to_i; zz_t.to_f; zz_t - zz_t; zz_t.strftime("%Y"); zz_t.year; zz_t.to_s; zz_t.inspect; Time.at(0); zz_t <=> zz_t; Marshal.load(Marshal.dump(zz_t)); zz_t + 1; Time.local(2000, 1, 1); zz_t.usec }
zz_step("zlib") { Zlib::Inflate.inflate(Zlib::Deflate.deflate("zz")) }
zz_step("regexp ops") { zz_r = Regexp.new("a(b)"); zz_m = zz_r.match("ab"); zz_m[1]; zz_m.pre_match; $1; zz_r.source; zz_r.inspect; Regexp.escape("a.b"); "ab" =~ zz_r; zz_r === "ab"; zz_m.to_a; Regexp.last_match }
zz_step("sleep") { sleep(0.01) }
zz_step("at_exit") { at_exit { } }
zz_step("require missing") { require "zz-no-such-library" }
zz_step("ObjectSpace.each_object") { ObjectSpace.each_object(Class) { } }
zz_step("method_missing") { zz_c = Class.new { def method_missing(zz_a, *zz_b); zz_a; end }; zz_c.new.zz_anything(1) }
zz_step("singleton and define_method") { zz_o = Object.new; def zz_o.zz_one; 1; end; class << zz_o; attr_reader :zz_two; end; zz_o.zz_one; ZZ_Thing.send(:define_method, :zz_two) { 2 }; ZZ_Thing.new.zz_two; zz_o.extend(Comparable) }
zz_step("exception details") { begin; raise ArgumentError, "zz"; rescue => zz_e; zz_e.message; zz_e.backtrace; zz_e.inspect; zz_e.to_s; zz_e.exception("y"); zz_e.set_backtrace([]); end; begin; raise RGSSError, "zz"; rescue RGSSError; end; begin; exit; rescue SystemExit => zz_e; zz_e.status; end; begin; raise Errno::EACCES; rescue SystemCallError => zz_e; zz_e.errno; end; begin; [].fetch(3); rescue IndexError; end; begin; :a.zz; rescue NoMethodError => zz_e; zz_e.name; zz_e.args; end; begin; loop { raise StopIteration }; rescue NameError, StandardError; end; begin; def zz_deep(zz_a); zz_deep(zz_a + 1); end; zz_deep(0); rescue SystemStackError; end }
zz_step("ensure retry redo next break") { zz_n = 0; begin; zz_n += 1; raise "zz" if zz_n < 2; rescue; retry; ensure; zz_n = 5; end; [1, 2].each { |zz_v| next if zz_v == 1; break }; while true; break; end; for zz_v in [1, 2]; end; zz_a, zz_b = 1, 2; zz_a, (zz_b, *zz_c) = 1, [2, 3]; case 5 when 1..3 then 1 when Integer then 2 else 3 end; zz_a ||= 3; zz_d = [1]; zz_d[0] += 1; zz_d << 2 if not zz_d.empty?; yield_self = 1 unless false }

# --- RGSS2 values
zz_step("Color.new") { $zz_color = Color.new(1, 2, 3, 4) }
zz_step("Color ops") { $zz_color.red; $zz_color.green = 5; $zz_color.set(1, 2, 3); $zz_color.alpha; $zz_color.inspect; $zz_color.to_s; $zz_color == Color.new(1, 2, 3); $zz_color.dup; $zz_color.clone; Color.new(1, 2, 3) }
zz_step("Color marshal") { Marshal.load(Marshal.dump($zz_color)) }
zz_step("Color _dump _load") { Color._load($zz_color._dump(0)) }
zz_step("Tone.new") { $zz_tone = Tone.new(1, 2, 3, 4) }
zz_step("Tone ops") { $zz_tone.red; $zz_tone.gray = 5; $zz_tone.set(1, 2, 3); $zz_tone.inspect; $zz_tone == Tone.new(1, 2, 3); $zz_tone.dup; $zz_tone.clone }
zz_step("Tone marshal") { Marshal.load(Marshal.dump($zz_tone)) }
zz_step("Rect.new") { $zz_rect = Rect.new(1, 2, 3, 4) }
zz_step("Rect ops") { $zz_rect.x; $zz_rect.width = 5; $zz_rect.set(1, 2, 3, 4); $zz_rect.empty; $zz_rect.inspect; $zz_rect == Rect.new(1, 2, 3, 4); $zz_rect.dup; $zz_rect.clone }
zz_step("Rect marshal") { Marshal.load(Marshal.dump($zz_rect)) }
zz_step("Table.new") { $zz_table = Table.new(4, 4, 3) }
zz_step("Table ops") { $zz_table[1, 1, 1] = 5; $zz_table[1, 1, 1]; $zz_table.xsize; $zz_table.ysize; $zz_table.zsize; $zz_table.resize(5, 5, 3); $zz_table.dup; $zz_table.clone; Table.new(3); Table.new(3, 3); $zz_table.inspect }
zz_step("Table marshal") { Marshal.load(Marshal.dump($zz_table)) }
zz_step("Font.new") { $zz_font = Font.new }
zz_step("Font.new name size") { Font.new("Arial", 20); Font.new(["Arial", "Verdana"]) }
zz_step("Font readers") { $zz_font.name; $zz_font.size; $zz_font.bold; $zz_font.italic; $zz_font.shadow }
zz_step("Font#color") { $zz_font.color }
zz_step("Font#color=") { $zz_font.color = Color.new(1, 2, 3) }
zz_step("Font writers") { $zz_font.name = "Arial"; $zz_font.size = 20; $zz_font.bold = true; $zz_font.italic = true; $zz_font.shadow = false; $zz_font.name = ["Arial"] }
zz_step("Font defaults read") { Font.default_name; Font.default_size; Font.default_bold; Font.default_italic; Font.default_shadow; Font.default_color }
zz_step("Font defaults write") { Font.default_name = Font.default_name; Font.default_size = 20; Font.default_bold = false; Font.default_italic = false; Font.default_shadow = true; Font.default_color = Color.new(255, 255, 255) }
zz_step("Font.exist?") { Font.exist?("Arial"); Font.exist?("zz no font") }
zz_step("Font dup inspect") { $zz_font.dup; $zz_font.clone; $zz_font.inspect }
zz_step("Font marshal") { Marshal.dump($zz_font) }

# --- bitmaps
zz_step("Bitmap.new w h") { $zz_bitmap = Bitmap.new(64, 48) }
zz_step("Bitmap.new w h again") { $zz_bitmap2 = Bitmap.new(32, 32) }
zz_step("Bitmap.new file") { $zz_skin = Bitmap.new("Graphics/System/Window") }
zz_step("Bitmap.new missing file") { Bitmap.new("Graphics/System/zz-missing") }
zz_step("Bitmap size readers") { $zz_bitmap.width; $zz_bitmap.height; $zz_bitmap.disposed? }
zz_step("Bitmap#rect") { $zz_bitmap.rect }
zz_step("Bitmap#font") { $zz_bitmap.font }
zz_step("Bitmap#font.color") { $zz_bitmap.font.color }
zz_step("Bitmap#font=") { $zz_bitmap.font = Font.new }
zz_step("Bitmap#font.color=") { $zz_bitmap.font.color = Color.new(9, 9, 9) }
zz_step("Bitmap pixels") { $zz_bitmap.set_pixel(1, 1, Color.new(1, 2, 3)); $zz_bitmap.get_pixel(1, 1) }
zz_step("Bitmap fills") { $zz_bitmap.fill_rect(0, 0, 4, 4, Color.new(1, 2, 3)); $zz_bitmap.fill_rect(Rect.new(0, 0, 4, 4), Color.new(1, 2, 3)); $zz_bitmap.gradient_fill_rect(0, 0, 4, 4, Color.new(1, 2, 3), Color.new(4, 5, 6)); $zz_bitmap.gradient_fill_rect(Rect.new(0, 0, 4, 4), Color.new(1, 2, 3), Color.new(4, 5, 6), true); $zz_bitmap.clear_rect(0, 0, 2, 2); $zz_bitmap.clear_rect(Rect.new(0, 0, 2, 2)); $zz_bitmap.clear }
zz_step("Bitmap blt") { $zz_bitmap.blt(0, 0, $zz_bitmap2, Rect.new(0, 0, 8, 8)); $zz_bitmap.blt(0, 0, $zz_bitmap2, $zz_bitmap2.rect, 128); $zz_bitmap.stretch_blt(Rect.new(0, 0, 16, 16), $zz_bitmap2, Rect.new(0, 0, 8, 8)); $zz_bitmap.stretch_blt($zz_bitmap.rect, $zz_bitmap2, $zz_bitmap2.rect, 100) }
zz_step("Bitmap text_size") { $zz_bitmap.text_size("zz") }
zz_step("Bitmap draw_text") { $zz_bitmap.draw_text(0, 0, 60, 24, "zz"); $zz_bitmap.draw_text(Rect.new(0, 0, 60, 24), "zz", 1); $zz_bitmap.draw_text(0, 0, 60, 24, 12, 2) }
zz_step("Bitmap effects") { $zz_bitmap.hue_change(30); $zz_bitmap.blur; $zz_bitmap.radial_blur(10, 2) }
zz_step("Bitmap dup clone") { $zz_bitmap.dup.dispose; $zz_bitmap.clone.dispose }
zz_step("Bitmap inspect") { $zz_bitmap.inspect }
zz_step("Bitmap marshal") { Marshal.dump($zz_bitmap) }
zz_step("Bitmap dispose") { $zz_bitmap2.dispose; $zz_bitmap2.disposed? }
zz_step("Bitmap disposed use") { $zz_bitmap2.width }

# --- graphics
zz_step("Graphics readers") { Graphics.frame_rate; Graphics.frame_count; Graphics.brightness; Graphics.width; Graphics.height }
zz_step("Graphics writers") { Graphics.frame_rate = 60; Graphics.frame_count = 0; Graphics.brightness = 255 }
zz_step("Graphics.update") { Graphics.update }
zz_step("Graphics.update again") { Graphics.update }
zz_step("Graphics.frame_reset") { Graphics.frame_reset }
zz_step("Graphics.wait") { Graphics.wait(2) }
zz_step("Graphics.freeze") { Graphics.freeze }
zz_step("Graphics.transition") { Graphics.transition(2) }
zz_step("Graphics.transition file") { Graphics.freeze; Graphics.transition(2, "Graphics/System/BattleStart", 40) }
zz_step("Graphics.fadeout fadein") { Graphics.fadeout(2); Graphics.fadein(2) }
zz_step("Graphics.snap_to_bitmap") { $zz_snap = Graphics.snap_to_bitmap }
zz_step("Graphics.resize_screen") { Graphics.resize_screen(544, 416) }

# --- viewport
zz_step("Viewport.new 4") { $zz_viewport = Viewport.new(0, 0, 100, 100) }
zz_step("Viewport.new rect") { $zz_viewport2 = Viewport.new(Rect.new(0, 0, 50, 50)) }
zz_step("Viewport#rect") { $zz_viewport.rect }
zz_step("Viewport#rect=") { $zz_viewport.rect = Rect.new(0, 0, 90, 90) }
zz_step("Viewport#color") { $zz_viewport.color }
zz_step("Viewport#color=") { $zz_viewport.color = Color.new(1, 2, 3) }
zz_step("Viewport#tone") { $zz_viewport.tone }
zz_step("Viewport#tone=") { $zz_viewport.tone = Tone.new(1, 2, 3) }
zz_step("Viewport numbers") { $zz_viewport.visible = true; $zz_viewport.z = 5; $zz_viewport.ox = 1; $zz_viewport.oy = 1; $zz_viewport.visible; $zz_viewport.z; $zz_viewport.ox; $zz_viewport.oy }
zz_step("Viewport flash update") { $zz_viewport.flash(Color.new(1, 2, 3), 3); $zz_viewport.update; $zz_viewport.flash(nil, 3); $zz_viewport.update }
zz_step("Viewport inspect") { $zz_viewport.inspect; $zz_viewport.disposed? }
zz_step("Viewport dispose") { $zz_viewport2.dispose }

# --- sprite
zz_step("Sprite.new") { $zz_sprite = Sprite.new }
zz_step("Sprite.new viewport") { $zz_sprite2 = Sprite.new($zz_viewport) }
zz_step("Sprite#bitmap nil") { $zz_sprite.bitmap }
zz_step("Sprite#bitmap=") { $zz_sprite.bitmap = $zz_bitmap }
zz_step("Sprite#bitmap") { $zz_sprite.bitmap }
zz_step("Sprite#src_rect") { $zz_sprite.src_rect }
zz_step("Sprite#src_rect=") { $zz_sprite.src_rect = Rect.new(0, 0, 8, 8) }
zz_step("Sprite#viewport") { $zz_sprite.viewport; $zz_sprite2.viewport }
zz_step("Sprite#viewport=") { $zz_sprite.viewport = $zz_viewport; $zz_sprite.viewport = nil }
zz_step("Sprite#color") { $zz_sprite.color }
zz_step("Sprite#color=") { $zz_sprite.color = Color.new(1, 2, 3) }
zz_step("Sprite#tone") { $zz_sprite.tone }
zz_step("Sprite#tone=") { $zz_sprite.tone = Tone.new(1, 2, 3) }
zz_step("Sprite numbers") { $zz_sprite.visible = true; $zz_sprite.x = 1; $zz_sprite.y = 1; $zz_sprite.z = 1; $zz_sprite.ox = 1; $zz_sprite.oy = 1; $zz_sprite.zoom_x = 1.5; $zz_sprite.zoom_y = 1.5; $zz_sprite.angle = 10; $zz_sprite.mirror = true; $zz_sprite.bush_depth = 2; $zz_sprite.bush_opacity = 100; $zz_sprite.opacity = 200; $zz_sprite.blend_type = 1; $zz_sprite.wave_amp = 2; $zz_sprite.wave_length = 100; $zz_sprite.wave_speed = 300; $zz_sprite.wave_phase = 1.5 }
zz_step("Sprite readers") { $zz_sprite.visible; $zz_sprite.x; $zz_sprite.y; $zz_sprite.z; $zz_sprite.ox; $zz_sprite.oy; $zz_sprite.zoom_x; $zz_sprite.zoom_y; $zz_sprite.angle; $zz_sprite.mirror; $zz_sprite.bush_depth; $zz_sprite.bush_opacity; $zz_sprite.opacity; $zz_sprite.blend_type; $zz_sprite.wave_amp; $zz_sprite.wave_length; $zz_sprite.wave_speed; $zz_sprite.wave_phase; $zz_sprite.width; $zz_sprite.height }
zz_step("Sprite flash update") { $zz_sprite.flash(Color.new(1, 2, 3), 3); $zz_sprite.update; $zz_sprite.flash(nil, 3); $zz_sprite.update }
zz_step("Sprite inspect") { $zz_sprite.inspect; $zz_sprite.disposed? }
zz_step("Sprite subclass") { zz_c = Class.new(Sprite) { def initialize(zz_v); super(zz_v); @zz_attr = 1; end }; $zz_sprite3 = zz_c.new($zz_viewport) }
zz_step("Sprite draw") { Graphics.update }
zz_step("Sprite dispose") { $zz_sprite2.dispose; $zz_sprite2.disposed? }
zz_step("Sprite disposed use") { $zz_sprite2.x }

# --- plane
zz_step("Plane.new") { $zz_plane = Plane.new }
zz_step("Plane.new viewport") { $zz_plane2 = Plane.new($zz_viewport) }
zz_step("Plane#bitmap=") { $zz_plane.bitmap = $zz_bitmap; $zz_plane.bitmap }
zz_step("Plane#viewport") { $zz_plane.viewport; $zz_plane2.viewport; $zz_plane.viewport = $zz_viewport }
zz_step("Plane#color tone") { $zz_plane.color; $zz_plane.tone; $zz_plane.color = Color.new(1, 2, 3); $zz_plane.tone = Tone.new(1, 2, 3) }
zz_step("Plane numbers") { $zz_plane.visible = true; $zz_plane.z = 1; $zz_plane.ox = 1; $zz_plane.oy = 1; $zz_plane.zoom_x = 2.0; $zz_plane.zoom_y = 2.0; $zz_plane.opacity = 100; $zz_plane.blend_type = 1; $zz_plane.visible; $zz_plane.z; $zz_plane.zoom_x }
zz_step("Plane draw dispose") { Graphics.update; $zz_plane.dispose; $zz_plane2.dispose }

# --- window
zz_step("Window.new") { $zz_window = Window.new }
zz_step("Window.new viewport") { $zz_window2 = Window.new($zz_viewport) }
zz_step("Window#windowskin") { $zz_window.windowskin }
zz_step("Window#windowskin=") { $zz_window.windowskin = $zz_skin }
zz_step("Window#contents") { $zz_window.contents }
zz_step("Window#contents=") { $zz_window.contents = Bitmap.new(32, 32) }
zz_step("Window#cursor_rect") { $zz_window.cursor_rect }
zz_step("Window#cursor_rect=") { $zz_window.cursor_rect = Rect.new(0, 0, 16, 16) }
zz_step("Window#viewport") { $zz_window.viewport; $zz_window2.viewport; $zz_window.viewport = $zz_viewport }
zz_step("Window numbers") { $zz_window.active = true; $zz_window.visible = true; $zz_window.pause = true; $zz_window.x = 1; $zz_window.y = 1; $zz_window.width = 96; $zz_window.height = 96; $zz_window.z = 5; $zz_window.ox = 1; $zz_window.oy = 1; $zz_window.opacity = 200; $zz_window.back_opacity = 200; $zz_window.contents_opacity = 200; $zz_window.openness = 255 }
zz_step("Window readers") { $zz_window.active; $zz_window.visible; $zz_window.pause; $zz_window.x; $zz_window.y; $zz_window.width; $zz_window.height; $zz_window.z; $zz_window.ox; $zz_window.oy; $zz_window.opacity; $zz_window.back_opacity; $zz_window.contents_opacity; $zz_window.openness }
zz_step("Window update draw") { $zz_window.update; Graphics.update }
zz_step("Window subclass") { zz_c = Class.new(Window) { def initialize; super; @zz_attr = 1; self.windowskin = $zz_skin; end }; $zz_window3 = zz_c.new }
zz_step("Window inspect") { $zz_window.inspect; $zz_window.disposed? }
zz_step("Window dispose") { $zz_window2.dispose; $zz_window.dispose; $zz_window3.dispose }

# --- tilemap
zz_step("Tilemap.new") { $zz_tilemap = Tilemap.new }
zz_step("Tilemap.new viewport") { $zz_tilemap2 = Tilemap.new($zz_viewport) }
zz_step("Tilemap#bitmaps") { $zz_tilemap.bitmaps }
zz_step("Tilemap#bitmaps[0]") { $zz_tilemap.bitmaps[0] }
zz_step("Tilemap#bitmaps[0]=") { $zz_tilemap.bitmaps[0] = $zz_bitmap }
zz_step("Tilemap#bitmaps[8]=") { $zz_tilemap.bitmaps[8] = $zz_bitmap; $zz_tilemap.bitmaps[8] }
zz_step("Tilemap#map_data") { $zz_tilemap.map_data }
zz_step("Tilemap#map_data=") { $zz_tilemap.map_data = Table.new(20, 15, 3); $zz_tilemap.map_data }
zz_step("Tilemap#flash_data=") { $zz_tilemap.flash_data; $zz_tilemap.flash_data = Table.new(20, 15); $zz_tilemap.flash_data }
zz_step("Tilemap#passages=") { $zz_tilemap.passages; $zz_tilemap.passages = Table.new(8192); $zz_tilemap.passages }
zz_step("Tilemap#viewport") { $zz_tilemap.viewport; $zz_tilemap2.viewport; $zz_tilemap.viewport = $zz_viewport }
zz_step("Tilemap numbers") { $zz_tilemap.visible = true; $zz_tilemap.ox = 1; $zz_tilemap.oy = 1; $zz_tilemap.visible; $zz_tilemap.ox; $zz_tilemap.oy }
zz_step("Tilemap update draw") { $zz_tilemap.update; Graphics.update }
zz_step("Tilemap inspect") { $zz_tilemap.inspect; $zz_tilemap.disposed?; $zz_tilemap.bitmaps.inspect }
zz_step("Tilemap dispose") { $zz_tilemap.dispose; $zz_tilemap2.dispose }

# --- input, audio, data, the rest
zz_step("Input.update") { Input.update }
zz_step("Input queries") { Input.press?(Input::C); Input.trigger?(Input::B); Input.repeat?(Input::DOWN); Input.dir4; Input.dir8 }
zz_step("Audio stops") { Audio.bgm_stop; Audio.bgs_stop; Audio.me_stop; Audio.se_stop }
zz_step("Audio fades") { Audio.bgm_fade(100); Audio.bgs_fade(100); Audio.me_fade(100) }
zz_step("Audio.se_play") { Audio.se_play("Audio/SE/Cancel", 80, 100) }
zz_step("Audio.bgm_play missing") { Audio.bgm_play("Audio/BGM/zz-missing") }
zz_step("Audio.bgs_play me_play missing") { begin; Audio.bgs_play("Audio/BGS/zz-missing"); rescue Exception; end; Audio.me_play("Audio/ME/zz-missing") }
zz_step("save_data") { save_data([1, "a", ZZ_Thing.new, Color.new(1, 2, 3), Table.new(2)], "zz-save.rvdata") }
zz_step("load_data") { load_data("zz-save.rvdata") }
zz_step("load_data missing") { load_data("Data/zz-missing.rvdata") }
zz_step("Win32API.new") { $zz_api = Win32API.new("kernel32", "GetTickCount", "", "L") }
zz_step("Win32API#call") { $zz_api.call }
zz_step("Win32API.new second") { Win32API.new("user32", "GetSystemMetrics", "I", "I").call(0) }
zz_step("RPG classes") { RPG::Map.new(17, 13); RPG::Event.new(1, 1); RPG::BGM.new("", 100, 100); RPG::SE.new; RPG::Actor.new; RPG::MoveRoute.new; RPG::EventCommand.new; RPG::System.new; RPG::Animation.new }
zz_step("RPG::BGM play stop") { RPG::BGM.new("", 100, 100).play; RPG::BGM.stop; RPG::BGM.fade(100); RPG::BGS.stop; RPG::ME.stop; RPG::SE.stop; RPG::BGM.last }
zz_step("RPG marshal") { Marshal.load(Marshal.dump(RPG::Map.new(17, 13))) }
zz_step("NKF") { NKF.nkf("-w", "zz") }
zz_step("globals") { $DEBUG; $TEST; $BTEST; $KCODE; $RGSS_SCRIPTS.size; $:; $"; $stdout; $stderr; $; ; $, }
zz_step("Graphics after all") { 3.times { Graphics.update; Input.update } }
zz_step("nothing at the end") { }

zz_run
File.open("lazy-all.txt", "wb") { |zz_file| zz_file.write(Symbol.all_symbols.map { |zz_s| zz_s.to_i.to_s + " " + zz_s.to_s }.join("\n") + "\n") }
exit
