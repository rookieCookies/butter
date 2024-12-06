use crate::{engine::Engine, scene_manager::scene_tree::SceneTree};

pub struct Scene;

impl mlua::UserData for Scene {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("restart_game", |_, _: ()| {
            let engine = Engine::get();
            let entry = &engine.project_settings.world.entry_scene;
            engine.change_scene(entry);
            Ok(())
        });


        methods.add_function("load", |_, name: String| {
            let scene = SceneTree::from_file(&name);
            Ok(scene)
        });
    }
}


impl mlua::UserData for SceneTree {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("instantiate", |_, this, _: ()| {
            Ok(Engine::get().scene_manager.borrow_mut().current.instantiate(this))
        });
    }
}
