local upflap = Texture.from_rgbaf32("character/upflap.png")
local midflap = Texture.from_rgbaf32("character/midflap.png")
local downflap = Texture.from_rgbaf32("character/downflap.png")

local function update(self)
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
    SceneManager.restart_game()
end 


local function ready(self)
    self.rb = self:get_component("RigidBody")
    PhysicsServer.attach_collider_event(
        self:get_component("Collider").physics_collider,
        on_collision
    )
    self.rb.physics_rb.gravity_scale = 1.5

end


return {
    ready = ready,
    update = update,
    draw = draw,

    name = "Player",

    fields = {
        velocity = "@export vec3",
        speed = "@export float = 500.0",
        rb = "RigidBody",
        started = "bool",
    }
}
