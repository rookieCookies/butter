class_name = "Player"

started = false
velocity = Math.vec2(0.0, 0.0)
rb = not_set

local upflap = Texture.from_rgbaf32("character/upflap.png")
local midflap = Texture.from_rgbaf32("character/midflap.png")
local downflap = Texture.from_rgbaf32("character/downflap.png")

function _update(self)
    if not self.started then
        self.rb.physics_rb.velocity = Math.vec3(0.0, 0.0, 0.0)
        self.position = Math.vec3(0.0, 0.0, 0.0)
        return
    end

    self.position = Math.vec3(Math.lerp(self.position.x, -5.0, Time.delta), self.position.y, self.position.z)

    if Input.is_key_pressed("space") then
        self.rb.physics_rb.velocity = Math.vec3(0.0, 12.5, 0.0)
    end

    self.rotation = Math.lerp(self.rotation, -0.7, Time.delta * 3)
    if self.rb.physics_rb.velocity.y > 2 then
        self.sprite = upflap
        self.rotation = 0.7
    elseif self.rb.physics_rb.velocity.y > -2 then
        self.sprite = midflap
    else 
        self.sprite = downflap
    end

    if self.position.y < -12.5 or self.position.y > 12.5 then
        SceneManager.restart_game()
    end
end


local function on_collision(self, collision)
    local player = self:get_component("Player")
    if not player.started then return end

    if collision.parent:get_component("Pipe") then
        SceneManager.restart_game()
    end
end 


function _ready(self)
    self.rb = self:get_component("RigidBody")
    PhysicsServer.attach_collider_event(
        self:get_component("Collider").physics_collider,
        on_collision
    )
    self.rb.physics_rb.gravity_scale = 1.5

end

