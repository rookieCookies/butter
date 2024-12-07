local function ready(self)
    self.physics_collider = PhysicsServer.create_rect_collider(self, self.scale.x * self.scale_mult.x, Math.abs(self.scale.y * self.scale_mult.y))
end



return {
    ready = ready,
    update = update,
    draw = draw_func,

    name = "Collider",

    fields = {
        scale_mult = "@export vec3 = vector.create(1.0, 1.0, 1.0)",
        physics_collider = "any",

    }
}
