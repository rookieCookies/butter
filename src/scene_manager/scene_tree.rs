use genmap::GenMap;
use tracing::{info, trace};

use crate::{engine::Engine, math::vector::Vec2};

use super::{node::Node, NodeId};

#[derive(Clone, Debug)]
pub struct SceneTree {
    pub map: GenMap<Node>,
    root: Option<NodeId>,
}


impl SceneTree {
    pub fn new() -> Self {
        Self { map: GenMap::with_capacity(0), root: None }
    }


    pub fn insert(&mut self, node: Node) -> NodeId {
        NodeId(self.map.insert(node))
    }


    pub fn len(&self) -> usize { self.map.inner_unck().len() }


    pub fn queue_free(engine: &mut Engine, node: NodeId) {
        info!("calling queue free on {node:?}");

        // call free on everything
        let nodes = engine.with(|engine| {
            engine.scene_manager.tree.iter_vec(node)
        });

        for node in nodes.iter().copied() {
            engine.get_mut().scene_manager.tree
                .get_mut(node).queued_free = true;

            let comps = {
                let mut engine = engine.get_mut();
                let node = engine.scene_manager.tree.get_mut(node);
                node.components.iter()
            };

            for comp in comps {
                let (functions, userdata, path) = {
                    let mut engine = engine.get_mut();
                    let node = engine.scene_manager.tree.get_mut(node);
                    let userdata = node.userdata_of(comp);

                    let component = node.components.get(comp);
                    let script = component.script;
                    let script = engine.script_manager.script(script);

                    (
                        script.functions.clone(),
                        userdata,
                        script.path(),
                    )
                };


                functions.queue_free(path, userdata);
            }
        }


        info!("freed");

    }


    pub fn exists(&self, handle: NodeId) -> bool {
        self.map.get(handle.0).is_some()
    }


    pub fn get(&self, handle: NodeId) -> &Node {
        self.map.get(handle.0).unwrap()
    }


    pub fn get_mut(&mut self, handle: NodeId) -> &mut Node {
        self.map.get_mut(handle.0).unwrap()
    }


    pub fn set_global_position(&mut self, of: NodeId, mut pos: Vec2) {
        trace!("set global position of '{of:?}' to {pos}");

        let node = self.get(of);
        let mut target_parent = node.parent;

        while let Some(parent) = target_parent {
            let this = self.get(parent);

            pos.x /= this.properties.scale.x;
            pos.y /= this.properties.scale.y;
            pos.x -= this.properties.position.x;
            pos.y -= this.properties.position.y;

            target_parent = this.parent;
        }

        self.get_mut(of).properties.position = pos;
    }



    pub fn set_global_rotation(&mut self, of: NodeId, mut rot: f32) {
        trace!("set global rotation of '{of:?}' to {rot}");

        let node = self.get(of);
        let mut target_parent = node.parent;

        while let Some(parent) = target_parent {
            let this = self.get(parent);

            rot -= this.properties.rotation;

            target_parent = this.parent;
        }

        self.get_mut(of).properties.rotation = rot;
    }


    pub fn iter_vec_root(&self) -> Vec<NodeId> {
        let Some(root) = self.root
        else { return vec![] };

        self.iter_vec(root)
    }


    pub fn iter_vec(&self, root: NodeId) -> Vec<NodeId> {
        let mut stack = vec![root];
        let mut coll = vec![];


        while let Some(node) = stack.pop() {
            coll.push(node);

            let node = self.get(node);
            stack.extend_from_slice(&node.children);
        }

        coll.reverse();

        coll
    }


    pub fn root(&self) -> Option<NodeId> {
        self.root
    }


    pub fn set_root(engine: &mut Engine, node: NodeId) {
        info!("set current scene root to {node:?}");

        let engine_ref = engine.get();
        let root = engine.get().scene_manager.tree.root;
        drop(engine_ref);

        if let Some(root) = root {
            Self::queue_free(engine, root);
        }

        engine.get_mut().scene_manager.tree.root = Some(node);
    }


    pub fn set_parent(&mut self, of: NodeId, to: Option<NodeId>) {
        let of_node = self.get_mut(of);
        let old_parent_id = of_node.parent;

        of_node.parent = to;

        if let Some(old_parent_id) = old_parent_id {
            let old_parent = self.get_mut(old_parent_id);
            let (index, _) = old_parent.children.iter()
                .enumerate()
                .find(|x| *x.1 == of)
                .unwrap();

            old_parent.children.remove(index);
        }


        if let Some(to) = to {
            let to_parent = self.get_mut(to);
            to_parent.children.push(of);
        }
    }
}

