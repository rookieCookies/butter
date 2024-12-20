pub mod fields;

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use fields::{Field, FieldId, FieldValue};
use mlua::AnyUserData;
use sokol::app::get_num_dropped_files;
use sti::{define_key, keyed::KVec};
use tracing::{error, info, trace, warn};

use crate::{asset_manager::TextureId, engine::Engine};

define_key!(u32, pub ScriptId);


#[derive(Debug)]
pub struct ScriptManager {
    pub scripts: KVec<ScriptId, Script>,
    pub path_to_script: HashMap<String, ScriptId>,
}


#[derive(Debug)]
pub struct Script {
    path  : &'static str,
    pub name  : String,
    pub fields: HashMap<String, FieldId>,
    pub default_fields: KVec<FieldId, Field>,
    pub functions: ScriptFunctions,
}


#[derive(Debug, Clone, Default)]
pub struct ScriptFunctions {
    ready : Option<mlua::Function>,
    update: Option<mlua::Function>,
    physics_update: Option<mlua::Function>,
    texture: Option<mlua::Function>,
    draw: Option<mlua::Function>,
    queue_free: Option<mlua::Function>,
}


impl ScriptManager {
    pub fn new() -> Self {
        let mut scripts = KVec::new();
        let functions = ScriptFunctions::default();

        scripts.push(Script {
            path: "<default>",
            name: String::new(),
            fields: HashMap::new(),
            default_fields: KVec::new(),
            functions
        });

 
        Self {
            scripts,
            path_to_script: HashMap::new(),
        }
   }


    pub fn load_current_dir(engine: &mut Engine) {
        info!("loading current directory scripts");

        let mut stack = vec![];
        stack.push(PathBuf::from_str("./").unwrap());
        while let Some(dir) = stack.pop() {
            let span = tracing::span!(tracing::Level::INFO, "searching dir ", path = dir.to_string_lossy().to_string());
            let _handle = span.entered();

            let read_dir = match dir.read_dir() {
                Ok(v) => v,
                Err(e) => {
                    error!("unable to read directory '{}': {}", dir.to_string_lossy(), e);
                    continue;
                },
            };

            for item in read_dir {
                let item = match item {
                    Ok(v) => v,
                    Err(e) => {
                        error!("unable to read an item: {}", e);
                        continue;
                    },
                }; 

                let path = item.path();
                trace!("found file: {}", path.to_string_lossy());

                let metadata = match item.metadata() {
                    Ok(v) => v,
                    Err(e) => {
                        error!("unable to retrieve metadata of '{}': {}", path.to_string_lossy(), e);
                        continue;
                    },
                };

                if metadata.file_type().is_dir() {
                    stack.push(path);
                    continue
                }

                let Some(ext) = path.extension()
                else { continue };

                if ext.to_str() == Some("lua") {
                    Self::from_path(engine, path.to_str().unwrap());
                }
            };
        }

        info!("loaded all scripts");
    }


    pub fn script(&self, script: ScriptId) -> &Script {
        &self.scripts[script]
    }
}


impl Script {
    pub fn new(path: String,
               fields: HashMap<String, FieldId>,
               default_fields: KVec<FieldId, Field>) -> Self {

        let name = fields.get("class_name")
            .map(|name| default_fields[*name].value.value().as_string_lossy())
            .map(|name| {
                if let Some(str) = name {
                    return Some(str)
                }

                warn!("the 'class_name' must be a string");

                None
            })
            .flatten()
            .unwrap_or_else(|| path.to_string());


        let get_func = |name: &str| {
            fields.get(name)
                .map(|index| {
                    let field = &default_fields[*index].value;
                    let func = field.value().as_function();
                    if let Some(func) = func {
                        return Some(func.clone())
                    }

                    warn!("the '{name}' must be a function");

                    None
                })
                .flatten()
        };


        let funcs = ScriptFunctions {
            ready: get_func("_ready"),
            update: get_func("_update"),
            physics_update: get_func("_physics_update"),
            texture: get_func("_create_texture"),
            draw: get_func("_draw"),
            queue_free: get_func("_queue_free"),
        };

        Self {
            path: path.leak(),
            name,
            fields,
            default_fields,
            functions: funcs,
        }
    }


    pub fn path(&self) -> &'static str {
        &self.path
    }
}


impl ScriptFunctions {
    pub fn update(&self, path: &str, user_data: AnyUserData) {
        let Some(update) = &self.update
        else { return };

        if let Err(e) = update.call::<()>(user_data) {
            error!("on update of '{}': \n{e}", path);
        }
    }


    pub fn ready(&self, path: &str, user_data: &AnyUserData) {
        let Some(ready) = &self.ready
        else { return };

        if let Err(e) = ready.call::<()>(user_data) {
            error!("on ready of '{}': \n{e}", path);
        }
    }


    pub fn draw(&self, path: &str, user_data: AnyUserData) {
        let Some(draw) = &self.draw
        else { return };

        if let Err(e) = draw.call::<()>(user_data) {
            error!("on draw of '{}': \n{e}", path);
        }
    }


    pub fn queue_free(&self, path: &str, user_data: AnyUserData) {
        let Some(queue_free) = &self.queue_free
        else { return };

        if let Err(e) = queue_free.call::<()>(user_data) {
            error!("on free of '{}': \n{e}", path);
        }
    }


    pub fn physics_update(&self, path: &str, user_data: AnyUserData) {
        let Some(physics_update) = &self.physics_update
        else { return };

        if let Err(e) = physics_update.call::<()>(user_data) {
            error!("on physics update of '{}': \n{e}", path);
        }
    }


    pub fn texture(&self, path: &str) -> Option<TextureId> {
        let Some(texture) = &self.texture
        else { return None };

        let ret = match texture.call::<mlua::Value>(()) {
            Ok(v) => v,
            Err(e) => {
                error!("on texture of '{}': \n{e}", path);
                return None;
            }
        };

        let Some(texture) = ret.as_userdata().map(|x| x.borrow::<TextureId>().ok()).flatten()
        else { 
            error!("the texture function of '{}' executes without error, \
                   but it doesn't return a texture id, it returns a '{}'",
                   path, ret.type_name());
            return None;
        };

        Some(*texture)
    }
}


impl Default for Script {
    fn default() -> Self {
        Self {
            path: "<default>",
            name: String::new(),
            fields: HashMap::new(),
            default_fields: KVec::new(),
            functions: ScriptFunctions::default(),
        }
    }
}


impl ScriptId {
    pub const EMPTY : Self = Self(0);
}
