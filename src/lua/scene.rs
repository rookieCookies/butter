use crate::{engine::Engine, scene_manager::{scene_template::TemplateScene, scene_tree::SceneTree}};

pub struct Scene;

impl mlua::UserData for Scene {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("restart_game", |_, _: ()| {
            let mut engine = Engine::generate();
            let entry = &Engine::project_settings().world.entry_scene;
            Engine::change_scene(&mut engine, entry);
            Ok(())
        });


        methods.add_function("load", |_, name: String| {
            let mut engine = Engine::generate();
            let scene = TemplateScene::from_file(&mut engine,
                                             &name);
            Ok(scene)
        });
    }
}


impl mlua::UserData for TemplateScene {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("instantiate", |_, this, _: ()| {
            println!("instantiate");
            Ok(this.instantiate(&mut Engine::generate()))
        });
    }
}
