use std::collections::{HashMap, HashSet};

use sti::keyed::KVec;
use tracing::{error, info, trace};

use crate::{engine::Engine, script_manager::{fields::{Field, FieldValue}, Script, ScriptId, ScriptManager}};

pub mod template_scene;


impl ScriptManager {
    /// Loads a script from a given path and then puts it into the
    /// Engine's script manager returning a script id to it
    ///
    /// Returns ScriptId::EMPTY
    pub fn from_path(engine: &mut Engine, path: &str) -> ScriptId {
        let span = tracing::span!(tracing::Level::ERROR, "loading script ", path);
        let _handle = span.entered();

        trace!("loading script");

        let engine_ref = engine.get();
        let sm = &engine_ref.script_manager;

        if let Some(script) = sm.path_to_script.get(path) {
            trace!("script is already loaded");
            return *script;
        }


        let Ok(file) = std::fs::read(path)
        else {
            error!("unable to read '{path}'");
            return ScriptId::EMPTY;
        };


        let Ok(lua) = std::str::from_utf8(&file)
        else {
            error!("'{path}' is not a valid utf-8 string");
            return ScriptId::EMPTY;
        };


        drop(engine_ref);

        Self::from_lua(engine, path, lua)
    }


    pub fn from_lua(engine: &mut Engine, path: &str, lua_file: &str) -> ScriptId {
        // we save the environment so we can diff it
        let environment = {
            let mut hashset = HashSet::new();
            let globals = Engine::lua().globals();

            for item in globals.pairs::<String, mlua::Value>() {
                let (key, _) = item.unwrap();
                hashset.insert(key);
            }

            hashset
        };


        let lua_chunk = Engine::lua().load(lua_file);
        let lua_result = lua_chunk.call::<mlua::Value>(());

        if let Err(e) = lua_result {
            error!("while executing the script: \n{e}");
            return ScriptId::EMPTY
        }


        let globals = Engine::lua().globals();

        let mut default_fields = KVec::new();
        let mut fields = HashMap::new();

        for item in globals.pairs::<String, mlua::Value>() {
            let (name, value) = item.unwrap();

            // if it existed in the old environment
            // keep it
            if environment.contains(&*name) {
                continue;
            }

            // if not, then it's a field so we remove it
            globals.raw_remove(&*name).unwrap();

            let field_value = FieldValue::new(value);
            let field = Field::new(name.clone(), field_value);

            let key = default_fields.push(field);
            fields.insert(name, key);
        }


        let script = Script::new(
            path.to_string(),
            fields,
            default_fields,
        );

        let name = script.name.clone();

        
        let mut engine = engine.get_mut();
        let sm = &mut engine.script_manager;

        let id = sm.scripts.push(script);

        if let Some(binded) = sm.path_to_script.get(&name) {
            let name_scr = sm.scripts.get(*binded).unwrap();
            error!("the name '{:?}' is already binded to '{}'", name, name_scr.path());

        } else {
            sm.path_to_script.insert(name.clone(), id);

        }

        sm.path_to_script.insert(path.to_string(), id);


        ScriptId::EMPTY
    }
}
