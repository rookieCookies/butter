use mlua::UserData;

pub struct Engine;

impl UserData for Engine {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("show_debug_colliders", |_, _| {
            Ok(crate::EngineHandle::generate().get_mut().show_colliders)
        });

        fields.add_field_method_set("show_debug_colliders", |_, _, value: bool| {
            crate::EngineHandle::generate().get_mut().show_colliders = value;
            Ok(())
        });
    }
}
