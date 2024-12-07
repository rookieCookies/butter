pub mod fields;

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use fields::{Field, FieldId};
use mlua::AnyUserData;
use sti::{define_key, keyed::KVec};
use tracing::{error, info, trace, warn};

use crate::{asset_manager::TextureId, engine::Engine};

define_key!(u32, pub ScriptId);


#[derive(Debug)]
pub struct ScriptManager {
    scripts: KVec<ScriptId, Script>,
    path_to_script: HashMap<String, ScriptId>,
}


#[derive(Debug)]
pub struct Script {
    path  : &'static str,
    pub name  : String,
    pub fields_ids: HashMap<String, FieldId>,
    pub fields_vec: KVec<FieldId, Field>,
    pub functions: ScriptFunctions,
}


#[derive(Debug, Clone, Default)]
pub struct ScriptFunctions {
    ready : Option<mlua::Function>,
    update: Option<mlua::Function>,
    texture: Option<mlua::Function>,
    draw: Option<mlua::Function>,
}



macro_rules! unwrap_lua {
    ($expr: expr, $ret: expr, $info: expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                error!("{}:\n{e}", $info);
                return $ret
            }
        }
        
    };
}


impl ScriptManager {
    pub fn new() -> Self {
        let mut scripts = KVec::new();
        let functions = ScriptFunctions {
            ready: None,
            update: None,
            texture: None,
            draw: None,
        };

        scripts.push(Script { path: "<default>", name: String::new(), fields_ids: HashMap::new(), fields_vec: KVec::new(), functions });
 
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
                    Self::load_script(engine, path.to_str().unwrap());
                }
            };
        }


    }


    pub fn script(&self, script: ScriptId) -> &Script {
        &self.scripts[script]
    }


    pub fn load_script(engine: &mut Engine, path: &str) -> ScriptId {
        let span = tracing::span!(tracing::Level::ERROR, "loading script ", path);
        let _handle = span.entered();

        info!("loading script");

        let engine_ref = engine.get();
        let sm = &engine_ref.script_manager;

        if let Some(script) = sm.path_to_script.get(path) {
            info!("script is already loaded");
            return *script;
        }


        let Ok(canon) = std::fs::canonicalize(path)
        else {
            error!("unable to canonicalize '{path}'");
            return ScriptId::EMPTY;
        };

        let Ok(file) = std::fs::read(path)
        else {
            error!("unable to read '{path}'");
            return ScriptId::EMPTY;
        };

        // we drop the engine ref so the handle is free
        drop(engine_ref);
        trace!("calling lua");
        let chunk = Engine::lua().load(file);
        let properties = unwrap_lua!(chunk.call::<mlua::Value>(()), ScriptId::EMPTY,
        format!("while executing lua script '{path}'"));


        let mlua::Value::Table(properties) = properties
        else {
            error!("'{path}' executed successfully but it returned '{}', \
                    it should return a table with the script's properties", properties.type_name());
            return ScriptId::EMPTY;
        };


        let retrieve_prop = |name: &str| {
            let val = unwrap_lua!(properties.get::<mlua::Value>(name), None,
                                      format!("while trying to get the '{name}' function from the properties:"));
            properties.raw_remove(name).unwrap();
            Some(val)
        };


        let retrieve_func = |name: &str| {
            let val = retrieve_prop(name)?;

            if let mlua::Value::Function(func) = val {
                return Some(func);
            }

            if let mlua::Value::Nil = val { return None }

            error!("the '{name}' function can't be read as it is not a function but a '{}'", val.type_name());
            None
        };


        let retrieve_table = |name: &str| {
            let val = retrieve_prop(name)?;

            if let mlua::Value::Table(table) = val {
                return Some(table);
            }

            if let mlua::Value::Nil = val { return None }

            error!("the '{name}' table can't be read as it is not a table but a '{}'", val.type_name());
            None
        };


        let mut name = retrieve_prop("name").map(|x| {
            if x.is_nil() { return None }
            match x.as_string() {
                Some(v) => Some(v.to_string_lossy()),
                None => {
                    error!("the property 'name' exists but it's not a string but a '{}'", x.type_name());
                    None
                },
            }}).flatten().unwrap_or(path.to_string());

        if name.is_empty() {
            name = canon.to_string_lossy().to_string();
        }

        let name = name;

        let mut engine = engine.get_mut();
        let sm = &mut engine.script_manager;

        if let Some(name) = sm.path_to_script.get(&name) {
            let name_scr = sm.scripts.get(*name).unwrap();
            let cond = std::fs::canonicalize(name_scr.path())
                .map(|x| x == canon)
                .unwrap_or_else(|_| name_scr.path == canon.to_string_lossy());
            if cond {
                return *name;
            }
        }

        let ready = retrieve_func("ready");
        let update = retrieve_func("update");
        let texture = retrieve_func("texture");
        let draw = retrieve_func("draw");
        let fields = retrieve_table("fields");

        for entry in properties.pairs::<mlua::Value, mlua::Value>() {
            let Ok((key, _)) = entry
            else {
                error!("unable to read the entry '{entry:?}' as a key-value pair");
                continue;
            };

            let key = match key.to_string() {
                Ok(v) => v,
                Err(_) => format!("{key:?}"),
            };

            warn!("unused entry in properties '{}'", key);
        }


        let funcs = ScriptFunctions { ready, update, texture, draw };
        let script = Script { path: path.to_string().leak(), name: String::new(), fields_ids: HashMap::new(), fields_vec: KVec::new(), functions: funcs };

        let id = sm.scripts.push(script);
        if let Some(binded) = sm.path_to_script.get(&name) {
            let name_scr = sm.scripts.get(*binded).unwrap();

            error!("the name '{:?}' is already binded to '{}'", name, name_scr.path);
        } else {
            sm.path_to_script.insert(name.clone(), id);
        }

        sm.path_to_script.insert(path.to_string(), id);

        let fields = match fields {
            Some(v) => {
                let mut hashmap = HashMap::new();
                let mut kvec = KVec::new();
                for value in v.pairs::<mlua::Value, mlua::Value>() {
                    let (key, value) = value.unwrap();

                    let key = match key.as_string() {
                        Some(v) => v,
                        None => {
                            error!("the field name '{key:?}' must be a string, ignoring field");
                            continue;
                        },
                    };
                    trace!("reading field '{}'", key.to_string_lossy());

                    let field = Field::from_value(&Engine::lua(), key.to_string_lossy(), value);
                    hashmap.insert(key.to_string_lossy(), kvec.push(field));
                }

                (hashmap, kvec)
            },
            None => (HashMap::new(), KVec::new()),
        };

        let script = sm.scripts.get_mut(id).unwrap();
        script.fields_ids = fields.0;
        script.fields_vec = fields.1;
        script.name = name;

        id
    }
}


impl Script {
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
            fields_ids: HashMap::new(),
            fields_vec: KVec::new(),
            functions: ScriptFunctions::default(),
        }
    }
}


impl ScriptId {
    pub const EMPTY : Self = Self(0);
}
