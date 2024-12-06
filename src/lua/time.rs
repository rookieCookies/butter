use mlua::UserData;

use crate::engine::Engine;

pub(super) struct Time;


impl UserData for Time {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_function_get("delta", |_, _| {
            Ok(Engine::get().dt.get())
        });
        fields.add_field_function_get("now", |_, _| {
            Ok(Engine::get().now.get())
        });
    }
}
