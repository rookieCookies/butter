use mlua::UserData;

use crate::engine::{Engine, EngineHandle};

pub(super) struct Time;


impl UserData for Time {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_function_get("delta", |_, _| {
            Ok(EngineHandle::generate().get().dt)
        });
        fields.add_field_function_get("now", |_, _| {
            Ok(EngineHandle::generate().get().now)
        });
    }
}
