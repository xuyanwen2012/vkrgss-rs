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
    BUTTONS.each do |name|
      button = Input.const_get(name)
      @held[button] = names.include?(name) ? @held[button] + 1 : 0
    end
  end

  def self.held(button)
    @held[button]
  end

  # The direction of one axis: opposite directions cancel.
  def self.axis(low, high)
    low_held = @held[Input.const_get(low)]
    high_held = @held[Input.const_get(high)]
    return [0, 0] if low_held > 0 && high_held > 0
    return [low, low_held] if low_held > 0
    return [high, high_held] if high_held > 0
    [0, 0]
  end

  DIRECTIONS = { :DOWN => 2, :LEFT => 4, :RIGHT => 6, :UP => 8 }

  def self.dir4
    horizontal, horizontal_held = axis(:LEFT, :RIGHT)
    vertical, vertical_held = axis(:DOWN, :UP)
    return DIRECTIONS[vertical] if horizontal == 0 && vertical != 0
    return DIRECTIONS[horizontal] if vertical == 0 && horizontal != 0
    return 0 if horizontal == 0
    # Of two perpendicular directions, the one pressed more recently.
    horizontal_held < vertical_held ? DIRECTIONS[horizontal] : DIRECTIONS[vertical]
  end

  def self.dir8
    horizontal = axis(:LEFT, :RIGHT)[0]
    vertical = axis(:DOWN, :UP)[0]
    return dir4 if horizontal == 0 || vertical == 0
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
    held = OracleProbe.held(button)
    held == 1 || (held >= 16 && (held - 16) % 4 == 0)
  end

  def dir4
    OracleProbe.dir4
  end

  def dir8
    OracleProbe.dir8
  end
end
