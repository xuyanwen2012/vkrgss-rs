# Probe of the vkxp oracle harness (`cargo xtask oracle`), added in front of the scripts of
# a copy of the game. It runs in the original runtime and in vkxp, so it keeps to Ruby 1.8.1.
#
# Frames are counted as the host of vkxp numbers them: every `Graphics.update` is one frame
# and `Graphics.transition(d)` is d frames. `OracleProbe.frame` is the number of the frame
# the scripts are working on, counted from 0.
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

  # Called when `frames` more frames have been shown.
  def self.advance(frames)
    @frame += frames
    return if SIDE != "original" || CAPTURES.empty?
    while @captured < CAPTURES.size && CAPTURES[@captured] < @frame
      hold(CAPTURES[@captured])
      @captured += 1
    end
    exit if @captured == CAPTURES.size
  end

  # Asks the harness for a picture of the window and waits until it has taken it.
  def self.hold(frame)
    name = "capture-#{frame}.request"
    File.open(name, "w") { |file| file.write(frame.to_s) }
    sleep(0.01) while FileTest.exist?(name)
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
    @rep_count == 1 || (@rep_count >= 16 && (@rep_count - 16) % 4 == 0)
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
    alias_method :oracle_probe_transition, :transition
  end

  def update
    oracle_probe_update
    OracleProbe.advance(1)
  end

  def transition(*arguments)
    oracle_probe_transition(*arguments)
    duration = arguments.empty? ? 8 : arguments[0]
    OracleProbe.advance(duration < 0 ? 0 : duration)
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
