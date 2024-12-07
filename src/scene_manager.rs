use std::collections::HashMap;

use scene_tree::SceneTree;

use crate::{math::vector::Vec2, physics::PhysicsServer};

pub mod node;
pub mod scene_tree;


#[derive(Debug)]
pub struct SceneManager {
    pub scenes: HashMap<String, SceneTree>,
    pub current: SceneTree,
    pub physics: PhysicsServer,
}

impl SceneManager {
    pub fn new(gravity: Vec2) -> Self {
        Self { 
            scenes: HashMap::new(),
            current: SceneTree::new(),
            physics: PhysicsServer::new(gravity),
        }
    }


    pub fn register(&mut self, name: String, scene: SceneTree) {
        self.scenes.insert(name, scene);
    }


    /// NOTE: this does not call the ready functions of the nodes
    pub fn load(&mut self, scene: &str) -> bool {
        let Some(scene) = self.scenes.get(scene)
        else { return false };

        self.physics = PhysicsServer::new(self.physics.gravity);

        self.current = scene.clone();

        true
    }
}
