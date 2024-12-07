use mlua::Value;

use crate::engine::{Engine, EngineHandle};

pub struct LuaTexture;
impl mlua::UserData for LuaTexture {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("from_rgbaf32", |_, path: String| {
            Ok(EngineHandle::generate().get_mut().asset_manager.from_image(&path))
        });
    }

}


impl mlua::UserData for crate::asset_manager::TextureId {}

impl mlua::FromLua for crate::asset_manager::TextureId {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let Value::UserData(data) = value
        else { return Err(mlua::Error::RuntimeError(format!("'{value:?}' can't be assigned to a texture"))) };

        let Ok(data) = data.borrow::<crate::asset_manager::TextureId>()
        else { return Err(mlua::Error::RuntimeError(format!("'{data:?}' can't be assigned to a texture"))) };

        Ok(*data)
    }
}
