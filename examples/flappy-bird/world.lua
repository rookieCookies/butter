class_name = "World"
value = 0
player = not_set
pipe = not_set
started = false

function _ready(self)
    self.player = self:get_child(0):get_component("Player")
    self.pipe = self:get_child(1):get_component("Pipe")
end



function _update(self)
    if not self.started and Input.is_key_pressed("space") then
        self.started = true
        self.player.rb.physics_rb.velocity = Math.vec3(0.0, 5.0, 0.0)
        self.player.started = true
        self.pipe.started = true
    end

end

