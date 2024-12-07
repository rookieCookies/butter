local function ready(self)
    self.player = self:get_child(0):get_component("Player")
    self.pipe = self:get_child(1):get_component("Pipe")
end



local function update(self)
    print(self.started)
    print(Input.is_key_pressed("space"))
    if not self.started and Input.is_key_pressed("space") then
        self.started = true
        self.player.rb.physics_rb.velocity = Math.vec3(0.0, 5.0, 0.0)
        self.player.started = true
        self.pipe.started = true
    end

end



return {
    ready = ready,
    update = update,

    name = "World",

    fields = {
        value = "integer",

        player = "Player",
        pipe = "Pipe",
        started = "bool",
    }
}
