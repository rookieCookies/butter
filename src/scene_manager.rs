use std::collections::HashMap;

use genmap::Handle;
use node::ComponentId;
use scene_template::TemplateScene;
use scene_tree::SceneTree;
use sti::{define_key, keyed::KVec};
use tracing::{error, info};

use crate::{engine::Engine, math::vector::Vec2, physics::PhysicsServer};

pub mod node;
pub mod scene_template;
pub mod scene_tree;


define_key!(u32, pub TemplateId);


#[derive(Debug)]
pub struct SceneManager {
    pub templates: KVec<TemplateId, TemplateScene>,
    pub path_to_template: HashMap<String, TemplateId>,
    pub physics: PhysicsServer,
    pub tree: SceneTree,
    pub queue_change: Option<TemplateScene>,
    initialized: InitState,
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub Handle);


#[derive(Debug)]
pub enum InitState {
    NotInitialized(KVec<TemplateId, String>),
    Initialized,
}


impl SceneManager {
    pub fn new(gravity: Vec2) -> Self {
        Self { 
            path_to_template: HashMap::new(),
            physics: PhysicsServer::new(gravity),
            templates: KVec::new(),
            tree: SceneTree::new(),
            queue_change: None,
            initialized: InitState::NotInitialized(KVec::new()),
        }
    }


    pub fn template_from_file(engine: &mut Engine, path: &str) -> TemplateId {
        info!("loading template at '{path}'");
        {
            let mut engine = engine.get_mut();
            match &mut engine.scene_manager.initialized {
                InitState::NotInitialized(kvec) => {
                    info!("scene manager not initialized");
                    return kvec.push(path.to_string());
                },
                
                _ => (),
            }
        }

        info!("scene manager initialized");
        let scene = TemplateScene::from_file(engine, path);
        engine.with(|engine| engine.scene_manager.templates.push(scene))
    }


    pub fn init_templates(engine: &mut Engine) {
        info!("initializing scene manager templates");

        let mut engine_ref = engine.get_mut();

        let InitState::NotInitialized(kvec) = 
            &mut engine_ref.scene_manager.initialized
        else {
            error!("scene manager has already initialized templates");
            return;
        };


        let kvec = core::mem::replace(kvec, KVec::new());


        drop(engine_ref);

        let mut templates = KVec::new();

        for (id, path) in kvec.iter() {
            let template = TemplateScene::from_file(engine, &path);
            let given_id = templates.push(template);

            debug_assert_eq!(id, given_id);
        }

        engine.with(|engine| {
            assert!(engine.scene_manager.templates.is_empty());
            engine.scene_manager.templates = templates;
            engine.scene_manager.initialized = InitState::Initialized
        });
    }


    pub fn call_ready(engine: &mut Engine, root: NodeId) {
        info!("calling ready on '{root:?}'");

        let nodes = engine.with(|engine| engine.scene_manager.tree.iter_vec(root));

        for node in nodes {
            let mut comp_index = 0u32;

            loop {
                comp_index += 1;
                let comp_index = comp_index - 1;
                let comp_index = ComponentId::new_unck(comp_index);

                let (functions, userdata, path) = {
                    let mut engine = engine.get_mut();
                    let node = engine.scene_manager.tree.get_mut(node);
                    if comp_index.inner() >= node.components.len() as u32 {
                        break;
                    }

                    let component = node.components.get_mut(comp_index);
                    if component.is_ready {
                        continue;
                    }

                    component.is_ready = true;

                    let script = component.script;
                    let userdata = node.userdata_of(comp_index);
                    let script = engine.script_manager.script(script);

                    (
                        script.functions.clone(),
                        userdata,
                        script.path(),
                    )
                };


                functions.ready(path, &userdata);
            }
        }
    }


}


impl NodeId {
    pub const PLACEHOLDER : Self = Self(Handle { gen: usize::MAX, idx: usize::MAX });


    /// Creates a new `NodeId` with the generation of it being 0
    pub fn from_idx(idx: u32) -> Self {
        Self(Handle { gen: 0, idx: idx as usize })
    }


    pub fn idx(&self) -> usize { self.0.idx }
}
