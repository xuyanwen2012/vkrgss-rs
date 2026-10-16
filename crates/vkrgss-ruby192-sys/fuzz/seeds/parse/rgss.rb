class Scene_Example < Scene_Base
  def start
    super
    create_windows
    @sprite = Sprite.new
    @sprite.bitmap = Cache.system("Window")
  end
  def update
    super
    return unless Input.trigger?(:C)
    Sound.play_ok
    SceneManager.call(Scene_Map)
  end
  def terminate
    super
    @sprite.dispose if @sprite && !@sprite.disposed?
  end
end
$data_actors = load_data("Data/Actors.rvdata2") rescue nil
