class_name = "Collider"
scale_mult = vector.create(1.0, 1.0, 1.0)
physics_collider = not_set


function _ready(self)
    self.physics_collider = PhysicsServer.create_rect_collider(self, self.scale.x * self.scale_mult.x, Math.abs(self.scale.y * self.scale_mult.y))

end


function _queue_free(self)
    PhysicsServer.delete_collider(self.physics_collider)
end

