use mlua::{Error, FromLua, Function, UserData};
use rapier2d::math::Rotation;

use crate::{engine::Engine, math::vector::{Vec2, Vec3}, physics::{ColliderId, RigidBodyId}};

use super::node::NodeUserData;

pub struct Physics;

impl UserData for Physics {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("create_rect_collider", |lua, (node, width, height): (NodeUserData, f32, f32)| {
            let userdata = Engine::get().scene_manager.borrow_mut().physics.collider_cuboid(lua, node, Vec2::new(width, height)).1;
            Ok(userdata)
        });

        methods.add_function("create_dynamic_rigidbody", |lua, node: NodeUserData| {
            let userdata = Engine::get().scene_manager.borrow_mut().physics.create_dynamic_rigidbody(lua, node.0).1;
            Ok(userdata)
        });

        methods.add_function("create_static_rigidbody", |lua, _: ()| {
            let userdata = Engine::get().scene_manager.borrow_mut().physics.create_static_rigidbody(lua).1;
            Ok(userdata)
        });

        methods.add_function("create_kinematic_rigidbody", |lua, node: NodeUserData| {
            let userdata = Engine::get().scene_manager.borrow_mut().physics.create_kinematic_rigidbody(lua, node.0).1;
            Ok(userdata)
        });

        methods.add_function("attach_collider_to_rigidbody", |_, (cl, rb): (ColliderId, RigidBodyId)| {
            Engine::get().scene_manager.borrow_mut().physics.attach_collider_to_rigidbody(cl, rb);
            Ok(())
        });

        methods.add_function("attach_collider_event", |_, (cl, func): (ColliderId, Function)| {
            Engine::get().scene_manager.borrow_mut().physics.attach_collider_event(cl, func);
            Ok(())
        });
    }
}


impl UserData for ColliderId {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("position", |_, this| {
            let scene = Engine::get().scene_manager.borrow();
            let pos = scene.physics.get_collider(*this).translation();

            Ok(Vec2::new(pos.x, pos.y))
        });
    }
}


impl UserData for RigidBodyId {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("position", |_, this| {
            let scene = Engine::get().scene_manager.borrow();
            let pos = scene.physics.get_rb(*this).translation();

            Ok(Vec2::new(pos.x, pos.y))
        });

        fields.add_field_method_get("rotation", |_, this| {
            let physics = &Engine::get().scene_manager.borrow().physics;
            let rot = physics.get_rb(*this).rotation().angle();

            Ok(rot)
        });

        fields.add_field_method_get("velocity", |_, this| {
            let physics = &Engine::get().scene_manager.borrow().physics;
            let pos = physics.get_rb(*this).linvel();

            Ok(Vec2::new(pos.x, pos.y))
        });

        fields.add_field_method_get("mass", |_, this| {
            let physics = &Engine::get().scene_manager.borrow().physics;
            Ok(physics.get_rb(*this).mass())
        });

        fields.add_field_method_get("gravity_scale", |_, this| {
            let physics = &Engine::get().scene_manager.borrow().physics;
            Ok(physics.get_rb(*this).gravity_scale())
        });

        fields.add_field_method_set("rotation", |_, this, val: f32| {
            let mut scene = Engine::get().scene_manager.borrow_mut();
            let rb = scene.physics.get_rb_mut(*this);
            rb.set_rotation(Rotation::from_angle(val), true);
            Ok(())
        });

        fields.add_field_method_set("position", |_, this, val: Vec3| {
            let mut scene = Engine::get().scene_manager.borrow_mut();
            let rb = scene.physics.get_rb_mut(*this);
            let position = rapier2d::na::Vector2::new(val.x, val.y);
            rb.set_position(rapier2d::na::Isometry2::new(position, 0.0), true);
            Ok(())
        });

        fields.add_field_method_set("velocity", |_, this, val: Vec2| {
            let mut scene = Engine::get().scene_manager.borrow_mut();
            let rb = scene.physics.get_rb_mut(*this);
            rb.set_linvel(rapier2d::na::Vector2::new(val.x, val.y), true);
            Ok(())
        });

        fields.add_field_method_set("mass", |_, this, val| {
            let mut scene = Engine::get().scene_manager.borrow_mut();
            let rb = scene.physics.get_rb_mut(*this);
            rb.set_additional_mass(val, true);
            Ok(())
        });

        fields.add_field_method_set("gravity_scale", |_, this, val| {
            let mut scene = Engine::get().scene_manager.borrow_mut();
            let rb = scene.physics.get_rb_mut(*this);
            rb.set_gravity_scale(val, true);
            Ok(())
        });
    }

}


impl FromLua for ColliderId {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let Some(userdata) = value.as_userdata()
        else { return Err(Error::runtime(format!("expected a collider id found {value:?}"))) };

        Ok(*userdata.borrow::<Self>()?)
    }
}

impl FromLua for RigidBodyId {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let Some(userdata) = value.as_userdata()
        else { return Err(Error::runtime(format!("expected a collider id found {value:?}"))) };

        Ok(*userdata.borrow::<Self>()?)
    }
}
