use mlua::{Error, IntoLua, UserData, Value, Vector};
use rand::Rng;
use tracing::warn;

use crate::math::vector::{Vec2, Vec3, Vec4};

pub(super) struct Math;

impl UserData for Math {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("min", |_, (v1, v2): (Value, Value)| {
            Ok(match (&v1, &v2) {
                (Value::Integer(v1), Value::Integer(v2)) => Value::Integer(*v1.min(v2)),
                  (Value::Integer(v2), Value::Number(v1))
                | (Value::Number (v1), Value::Integer(v2)) => Value::Number(v1.min(*v2 as f64)),
                (Value::Number(v1), Value::Number(v2)) => Value::Number(v1.min(*v2)),
                (Value::Vector(v1), Value::Vector(v2)) => Value::Vector(Vector::new(v1.x().min(v2.x()),
                                                                      v1.y().min(v2.y()),
                                                                      v1.z().min(v2.z()))),
                _ => return Err(mlua::Error::runtime(format!("can't take the 'min' of a '{}' and '{}'", v1.type_name(), v2.type_name()))),
            })
        });


        methods.add_function("max", |_, (v1, v2): (Value, Value)| {
            Ok(match (&v1, &v2) {
                (Value::Integer(v1), Value::Integer(v2)) => Value::Integer(*v1.max(v2)),
                  (Value::Integer(v2), Value::Number(v1))
                | (Value::Number (v1), Value::Integer(v2)) => Value::Number(v1.max(*v2 as f64)),
                (Value::Number(v1), Value::Number(v2)) => Value::Number(v1.max(*v2)),
                (Value::Vector(v1), Value::Vector(v2)) => Value::Vector(Vector::new(v1.x().max(v2.x()),
                                                                      v1.y().max(v2.y()),
                                                                      v1.z().max(v2.z()))),
                _ => return Err(mlua::Error::runtime(format!("can't take the 'max' of a '{}' and '{}'", v1.type_name(), v2.type_name()))),
            })
        });


        methods.add_function("abs", |_, v1: Value| {
            Ok(match &v1 {
                Value::Integer(v) => Value::Integer(v.abs()),
                Value::Number(v) => Value::Number(v.abs()),

                _ => return Err(mlua::Error::runtime(format!("can't take the 'abs' of a '{}'", v1.type_name()))),
            })
        });


        methods.add_function("random", |_, _: ()| {
            Ok(rand::thread_rng().gen::<f32>())
        });


        methods.add_function("lerp", |_, (v1, v2, step): (Value, Value, f32)| {
            fn lerp(a: f32, b: f32, t: f32) -> f32 {
                (1.0-t)*a+t*b
            }

            if let (Value::Vector(v1), Value::Vector(v2)) = (&v1, &v2) {
                return Ok(Value::Vector(Vector::new(
                            lerp(v1.x(), v2.x(), step),
                            lerp(v1.y(), v2.y(), step),
                            lerp(v1.z(), v2.z(), step))))

            }

            let v1 = match v1 {
                Value::Integer(v) => v as f32,
                Value::Number(v) => v as f32,
                _ => return Err(Error::runtime(format!("the 'origin' value must either be a 'number' or an 'integer' but it is {}", v1.type_name())))
            };
            let v2 = match v2 {
                Value::Integer(v) => v as f32,
                Value::Number(v) => v as f32,
                _ => return Err(Error::runtime(format!("the 'origin' value must either be a 'number' or an 'integer' but it is {}", v2.type_name())))
            };

            Ok(Value::Number(lerp(v1, v2, step).into()))
        });

        methods.add_function("vec2", |_, (x, y): (f32, f32)| {
            Ok(Vec2::new(x, y))
        });
        methods.add_function("vec3", |_, (x, y, z): (f32, f32, f32)| {
            Ok(Vec3::new(x, y, z))
        });
        methods.add_function("vec4", |_, (x, y, z, w): (f32, f32, f32, f32)| {
            Ok(Vec4::new(x, y, z, w))
        });

    }
}


impl UserData for Vec4 {}


impl mlua::FromLua for Vec4 {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let Value::UserData(vec) = value
        else { return Err(mlua::Error::RuntimeError(format!("'{value:?}' can't be assigned to a vec4"))) };

        Ok(*vec.borrow::<Self>()?)
    }
}


impl IntoLua for Vec3 {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(Value::Vector(Vector::new(self.x, self.y, self.z)))
    }
}


impl mlua::FromLua for Vec3 {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let Value::Vector(vec) = value
        else { return Err(mlua::Error::RuntimeError(format!("'{value:?}' can't be assigned to a vec3"))) };

        Ok(Self::new(vec.x(), vec.y(), vec.z()))
    }
}


impl mlua::FromLua for Vec2 {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let Value::Vector(vec) = value
        else { return Err(mlua::Error::RuntimeError(format!("'{value:?}' can't be assigned to a vec3"))) };

        if vec.z() != 0.0 {
            warn!("{vec} is passed as a vec2 but the z value is '{}' \
                  this value is ignored.", vec.z());
        }

        Ok(Self::new(vec.x(), vec.y()))
    }
}


impl IntoLua for Vec2 {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(Value::Vector(Vector::new(self.x, self.y, 0.0)))
    }
}



