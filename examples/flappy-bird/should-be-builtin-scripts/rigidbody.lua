local function ready(self)
    self.collider = self:get_component("Collider")

    if self.collider == nil then
        error(string.format("no collider on rigidbody", self.kind))
        return
    end

    if self.kind == "dynamic" then
        self.physics_rb = PhysicsServer.create_dynamic_rigidbody(self)
    elseif self.kind == "kinematic" then
        self.physics_rb = PhysicsServer.create_kinematic_rigidbody(self)
    elseif self.kind == "static" then
        self.physics_rb = PhysicsServer.create_static_rigidbody(self)
    else
        error(string.format("invalid rigidbody type '%s'", self.kind))
        return
    end

    self.physics_rb.position = self.position
    self.physics_rb.rotation = self.rotation

    PhysicsServer.attach_collider_to_rigidbody(self.collider.physics_collider, self.physics_rb)
end


return {
    ready = ready,
    name = "RigidBody",

    fields = {
        collider = "Collider",
        kind = "str = \"dynamic\"",
        physics_rb = "any",
        velocity = "@export float",
    }
}
