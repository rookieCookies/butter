use mlua::{AnyUserData, Lua, Value};
use sti::define_key;
use tracing::{error, warn};

use crate::{engine::Engine, math::vector::Vec3};

use super::ScriptId;


define_key!(u32, pub FieldId);


#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
    pub value: FieldValue,
    pub export: bool,
}


#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FieldType {
    Float,
    Integer,
    Bool,
    String,

    Vec3,

    AnyTable,
    Script(ScriptId),
    #[default]
    Any
}



#[derive(Debug, Clone)]
pub enum FieldValue {
    Float(f64),
    Integer(i32),
    Bool(bool),
    String(mlua::String),

    Vec3(Vec3),

    Table(mlua::Table),
    Script(Option<AnyUserData>),
    Any(mlua::Value),
}



impl Field {
    pub fn from_value(lua: &Lua, name: String, value: mlua::Value) -> Field {
        match value {
            mlua::Value::String(v) => {
                let string = v.to_string_lossy();
                let mut str = string.as_str();

                let export = str.starts_with("@export");
                if export {
                    str = str.split_once("@export").unwrap().1;
                    str = str.trim();
                }

                let (ty, default) = if let Some((ty, def)) = str.split_once('=') {
                    let def = def.trim();
                    let ty = FieldType::from_str(ty.trim());

                    let def = lua.load(format!("return {def}")).call::<Value>(())
                        .map(|value| {
                            match (&ty, value) {
                                (FieldType::Float, Value::Integer(v)) => FieldValue::Float(v as f64),
                                (FieldType::Float, Value::Number(v)) => FieldValue::Float(v),
                                (FieldType::Integer, Value::Integer(v)) => FieldValue::Integer(v),
                                (FieldType::Vec3, Value::Vector(vector)) => FieldValue::Vec3(Vec3::new(vector.x(), vector.y(), vector.z())),
                                (FieldType::AnyTable, Value::Table(table)) => FieldValue::Table(table),
                                (FieldType::Script(_), Value::Nil) => FieldValue::Script(None),
                                (FieldType::String, Value::String(str)) => FieldValue::String(str),
                                (FieldType::Bool, Value::Boolean(b)) => FieldValue::Bool(b),
                                (FieldType::Any, v) => FieldValue::Any(v),
                                value => {
                                    error!("the field is of type '{:?}' but you've provided a \
                                           default value of type '{}'", ty, value.1.type_name());

                                    FieldValue::default(lua, &ty)
                                }
                            }

                        })
                        .unwrap_or_else(|_| {
                            error!("failed to run the default value of '{str}'");
                            FieldValue::default(lua, &ty)
                        });

                    (ty, def)
                } else {
                    let ty = FieldType::from_str(str);
                    let def = FieldValue::default(lua, &ty);
                    (ty, def)
                };

                Field { ty, export, value: default, name }
            },

            _ => {
                error!("the field's value must either be a string, \
                       but it is '{}'. defaulting to `@export any`", value.type_name());
                Field { ty: FieldType::Any, export: false, value: FieldValue::default(lua, &FieldType::Any), name }
            }
        }
    }
}


impl FieldType {
    pub fn from_str(str: &str) -> Self {
        match str {
            "float" => FieldType::Float,
            "integer" => FieldType::Integer,
            "bool" => FieldType::Bool,
            "str" => FieldType::String,

            "vec3" => FieldType::Vec3,

            "table" => FieldType::AnyTable,
            "any" => FieldType::Any,

            _ => {
                warn!("type checking on component fields for other components is not implemented yet");
                FieldType::Any
                // FieldType::Script(script_manager.load_script(str))
            }
        }
    }
}


impl FieldValue {
    pub fn default(lua: &Lua, ty: &FieldType) -> Self {
        match ty {
            FieldType::Float => Self::Float(0.0),
            FieldType::Integer => Self::Integer(0),
            FieldType::Vec3 => Self::Vec3(Vec3::ZERO),
            FieldType::AnyTable => Self::Table(lua.create_table().unwrap()),
            FieldType::Script(_) => Self::Script(None),
            FieldType::Any => Self::Any(Value::Nil),
            FieldType::Bool => Self::Bool(false),
            FieldType::String => Self::String(Engine::lua().create_string("").unwrap()),
        }
    }
}

