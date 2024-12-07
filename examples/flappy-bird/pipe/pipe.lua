local function update(self)
    if not self.started then
        return
    end

    self.position -= Math.vec3(10 * self.speed * Time.delta, 0.0, 0.0)

    if self.position.x < -18 then
        self.speed *= 1.025
        self.position = Math.vec3(25, math.random() * 20 - 10, 0.0)
    end
end


return {
    ready = ready,
    update = update,

    name = "Pipe",

    fields = {
        velocity = "@export vec3",
        speed = "@export float = 500.0",
        rb = "RigidBody",
        started = "bool",
        speed = "float = 1.0"
    }
}
