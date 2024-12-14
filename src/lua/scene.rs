use crate::{engine::Engine, scene_manager::{scene_template::TemplateScene, SceneManager, TemplateId}};

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
            let scene = SceneManager::template_from_file(&mut Engine::generate(),
                                                         &name); 
            Ok(scene)
        });
    }
}


impl mlua::UserData for TemplateId {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("instantiate", |_, this, _: ()| {
            println!("instantiate");
            Ok(TemplateScene::instantiate(&mut Engine::generate(), *this))
        });
    }
}
