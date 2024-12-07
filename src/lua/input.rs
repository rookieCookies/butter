use mlua::Value;

use crate::{engine::Engine, event_manager::Keycode};

pub struct Input;

impl mlua::UserData for Input {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("is_key_down", |_, key: Keycode| {
            Ok(Engine::generate().get().input_manager.is_key_down(key))
        });

        methods.add_function("is_key_up", |_, key: Keycode| {
            Ok(Engine::generate().get().input_manager.is_key_up(key))
        });

        methods.add_function("is_key_pressed", |_, key: Keycode| {
            Ok(Engine::generate().get().input_manager.is_key_just_pressed(key))
        });

        methods.add_function("is_key_released", |_, key: Keycode| {
            Ok(Engine::generate().get().input_manager.is_key_just_released(key))
        });

        methods.add_function("get_axis", |_, (pos, neg): (Keycode, Keycode)| {
            Ok(Engine::generate().get().input_manager.get_axis(pos, neg))
        });

        methods.add_function("get_vector", |_, (pos_x, neg_x, pos_y, neg_y): (Keycode, Keycode, Keycode, Keycode)| {
            Ok(Engine::generate().get().input_manager.get_vector(pos_x, neg_x, pos_y, neg_y))
        });
    }
}


impl mlua::FromLua for Keycode {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        if let Value::String(value) = &value {
            if let Some(key) = Keycode::from_str(&value.to_str().unwrap()) {
                return Ok(key)
            }
        }
        return Err(mlua::Error::RuntimeError(
                format!("'{value:?}' is not a valid key code")));
    }
}


