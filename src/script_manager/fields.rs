use sti::define_key;

define_key!(u32, pub FieldId);


#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub value: FieldValue,
}


#[derive(Debug, Clone)]
pub struct FieldValue {
    lua_value: mlua::Value,
}


impl Field {
    pub fn new(name: String, value: FieldValue) -> Field {
        Field {
            name,
            value,
        }
    }
}


impl FieldValue {
    pub fn new(value: mlua::Value) -> FieldValue {
        FieldValue {
            lua_value: value,
        }
    }


    pub fn value(&self) -> &mlua::Value {
        &self.lua_value
    }

    pub fn value_mut(&mut self) -> &mut mlua::Value {
        &mut self.lua_value
    }
}
