# Probe of the vkrgss oracle harness (`cargo xtask oracle`) for games of RGSS2, added in front
# of the scripts of a copy of the game. It runs in the original runtime and in vkrgss, so it
# keeps to Ruby 1.8.1.
#
# Frames are counted as the original advances `Graphics.frame_count`, which the probe does not
# read, because the game may assign it: `update` is one frame, `wait(n)`, `fadeout(n)` and
# `fadein(n)` are n, and `transition(n)` is n after a `freeze` and none without one.
# `OracleProbe.frame` is the number of the frame the scripts are working on, counted from 0.
module OracleProbe
  # A string, not a symbol: both sides must intern exactly the same names, or the symbol
  # numbers of everything the game creates afterwards would differ between the two runs.
  SIDE = "__SIDE__"
  # Pairs of a frame and the buttons held from that frame on, in ascending order.
  SCHEDULE = [__SCHEDULE__]
  # The frames to capture, in ascending order.
  CAPTURES = [__CAPTURES__]
  BUTTONS = [:DOWN, :LEFT, :RIGHT, :UP, :A, :B, :C, :X, :Y, :Z, :L, :R,
             :SHIFT, :CTRL, :ALT, :F5, :F6, :F7, :F8, :F9]

  @frame = 0
  @captured = 0
  # Whether `Graphics.freeze` was called and no transition has followed it yet.
  @frozen = false
  # By the value of a button, the number of updates it has been held for.
  @held = Hash.new(0)
  # The one button that repeats, the one triggered most recently, and the number of updates
  # it has been held for since.
  @rep_button = nil
  @rep_count = 0
  # Which direction a diagonal gives `dir4`: 0 the horizontal one, 1 the vertical one.
  @axis_flag = 0
  @dir4 = 0

  def self.frame
    @frame
  end

  def self.original?
    SIDE == "original"
  end

  # Whether the frame that the next update shows is one to capture.
  def self.capture_next?
    original? && @captured < CAPTURES.size && CAPTURES[@captured] == @frame
  end

  # Called when `frames` more frames have been shown. `transition` says that a transition
  # showed them: the original cannot be held inside one, so a frame to capture that is not
  # its last one is captured late, with the picture the transition ends with.
  def self.advance(frames, transition)
    @frame += frames if frames > 0
    return if !original? || CAPTURES.empty?
    while @captured < CAPTURES.size && CAPTURES[@captured] < @frame
      hold(CAPTURES[@captured], transition && CAPTURES[@captured] < @frame - 1)
      @captured += 1
    end
    exit if @captured == CAPTURES.size
  end

  # Asks the harness for a picture of the window and waits until it has taken it.
  def self.hold(frame, late)
    name = "capture-#{frame}.request"
    File.open(name, "w") { |file| file.write(late ? "#{frame} late" : frame.to_s) }
    sleep(0.01) while FileTest.exist?(name)
    # The hold was a stall, after which the original returns from updates without drawing.
    Graphics.frame_reset
  end

  def self.freeze
    @frozen = true
  end

  # Called after a transition of `duration` frames.
  def self.transition(duration)
    shown = @frozen ? duration : 0
    @frozen = false
    advance(shown, true)
  end

  def self.update_input
    names = []
    SCHEDULE.each { |entry| names = entry[1] if entry[0] <= @frame }
    # In ascending order of the buttons' numbers: of several triggered in one update the
    # highest is the one that repeats.
    BUTTONS.each do |name|
      button = Input.const_get(name)
      @held[button] = names.include?(name) ? @held[button] + 1 : 0
      if @held[button] == 1
        @rep_button = button
        @rep_count = 0
      end
    end
    if @rep_button && @held[@rep_button] > 0
      @rep_count += 1
    else
      @rep_button = nil
      @rep_count = 0
    end
    update_dir4
  end

  def self.held(button)
    @held[button]
  end

  def self.repeat?(button)
    return false if button != @rep_button
    @rep_count == 1 || (@rep_count >= 24 && (@rep_count - 24) % 6 == 0)
  end

  # The direction held on one axis, or nil: opposite directions cancel.
  def self.axis(low, high)
    low_held = @held[Input.const_get(low)] > 0
    high_held = @held[Input.const_get(high)] > 0
    return nil if low_held == high_held
    low_held ? low : high
  end

  DIRECTIONS = { :DOWN => 2, :LEFT => 4, :RIGHT => 6, :UP => 8 }

  # A diagonal gives the direction the flag chooses and leaves the flag alone; a horizontal
  # direction held alone sets the flag and a vertical one clears it.
  def self.update_dir4
    horizontal = axis(:LEFT, :RIGHT)
    vertical = axis(:DOWN, :UP)
    if horizontal && vertical
      @dir4 = DIRECTIONS[@axis_flag == 0 ? horizontal : vertical]
    elsif horizontal
      @axis_flag = 1
      @dir4 = DIRECTIONS[horizontal]
    elsif vertical
      @axis_flag = 0
      @dir4 = DIRECTIONS[vertical]
    else
      @dir4 = 0
    end
  end

  def self.dir4
    @dir4
  end

  def self.dir8
    horizontal = axis(:LEFT, :RIGHT)
    vertical = axis(:DOWN, :UP)
    return dir4 if horizontal.nil? || vertical.nil?
    (vertical == :DOWN ? 0 : 6) + (horizontal == :LEFT ? 1 : 3)
  end
end

srand(__SEED__)

class << Graphics
  unless method_defined?(:oracle_probe_update)
    alias_method :oracle_probe_update, :update
    alias_method :oracle_probe_wait, :wait
    alias_method :oracle_probe_fadeout, :fadeout
    alias_method :oracle_probe_fadein, :fadein
    alias_method :oracle_probe_freeze, :freeze
    alias_method :oracle_probe_transition, :transition
  end

  def update
    # An update that comes late is not drawn, and the frame to capture must be.
    frame_reset if OracleProbe.capture_next?
    oracle_probe_update
    OracleProbe.advance(1, false)
  end

  # `wait`, `fadeout` and `fadein` of the original cannot be held at a frame inside them. On
  # its side they are the loops of updates that were measured to show the same pictures
  # frame for frame; vkrgss runs its own.
  def wait(duration)
    if OracleProbe.original?
      duration.times { update }
    else
      oracle_probe_wait(duration)
      OracleProbe.advance(duration, false)
    end
  end

  def fadeout(duration)
    if OracleProbe.original?
      from = brightness
      1.upto(duration) do |step|
        self.brightness = from - from * step / duration
        update
      end
      self.brightness = 0
    else
      oracle_probe_fadeout(duration)
      OracleProbe.advance(duration, false)
    end
  end

  def fadein(duration)
    if OracleProbe.original?
      from = brightness
      1.upto(duration) do |step|
        self.brightness = from + (255 - from) * step / duration
        update
      end
      self.brightness = 255
    else
      oracle_probe_fadein(duration)
      OracleProbe.advance(duration, false)
    end
  end

  def freeze
    oracle_probe_freeze
    OracleProbe.freeze
  end

  def transition(*arguments)
    oracle_probe_transition(*arguments)
    OracleProbe.transition(arguments.empty? ? 10 : arguments[0])
  end
end

class << Input
  def update
    OracleProbe.update_input
  end

  def press?(button)
    OracleProbe.held(button) > 0
  end

  def trigger?(button)
    OracleProbe.held(button) == 1
  end

  def repeat?(button)
    OracleProbe.repeat?(button)
  end

  def dir4
    OracleProbe.dir4
  end

  def dir8
    OracleProbe.dir8
  end
end
