use std::collections::HashMap;

use genmap::{GenMap, Handle};
use sti::{define_key, keyed::Key};
use tracing::info;

use crate::{engine::Engine, math::vector::Vec2, scene_manager::node::ComponentId, script_manager::{ScriptId, ScriptManager}};

use super::node::Node;

define_key!(u32, pub NodeIndex);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(Handle);

#[derive(Clone, Debug)]
pub struct SceneTree {
    pub map: GenMap<Node>,
}


impl SceneTree {
    pub fn new() -> Self {
        Self { map: GenMap::with_capacity(0) }
    }


    pub fn insert(&mut self, node: Node) -> NodeId {
        NodeId(self.map.insert(node))
    }


    pub fn trim(&self) -> SceneTree {
        let mut new = SceneTree::new();
        let mut hm = HashMap::new();

        for handle in self.map.iter() {
            let node = self.map.get(handle).unwrap();
            let new_handle = new.insert(node.clone());
            hm.insert(NodeId(handle), new_handle);
        }

        for new_handle in hm.values() {
            let new_handle = hm.get(new_handle).unwrap();
            let new_node = new.map.get_mut(new_handle.0).unwrap();

            for c in new_node.children.iter_mut() {
                *c = *hm.get(c).unwrap();
            }
        }

        new
    }


    pub fn instantiate(engine: &mut Engine, scene: &SceneTree) -> NodeId {
        let mut hashmap = HashMap::new();
        let mut engine_ref = engine.get_mut();
        let this = &mut engine_ref.scene_manager.current;


        let mut stack = vec![];
        if let Some(root) = scene.root() {
            stack.push(root);
        }


        while let Some(node_id) = stack.pop() {
            let node = scene.get(node_id);

            let insert_node = Node {
                properties: node.properties,
                children: vec![],
                parent: None,
                components: node.components.clone(),
                userdata: None,
                node_id // placeholder,
            };


            let insert_id = this.map.insert(insert_node.into());
            let insert_node_id = NodeId(insert_id);
            hashmap.insert(node_id, insert_node_id);

            if let Some(parent) = node.parent {
                this.set_parent(insert_node_id, Some(*hashmap.get(&parent).unwrap()));
            }

            this.get_mut(insert_node_id).node_id = insert_node_id;

            stack.extend_from_slice(&node.children);
        }

        drop(engine_ref);

        let nodes = scene.iter_vec();

        info!("ready all nodes");
        for node in nodes {
            let node = hashmap.get(&node).unwrap().clone();
            let mut comp_index = 0u32;
            loop {
                comp_index += 1;
                let comp_index = comp_index - 1;

                let (functions, userdata, path) = {
                    let mut engine = engine.get_mut();
                    let node = engine.scene_manager.current.get_mut(node);
                    if comp_index >= node.components.len() as u32 {
                        break;
                    }

                    let component = node.components.get_mut_index(comp_index);
                    if component.is_ready {
                        continue;
                    }

                    component.is_ready = true;

                    let script = component.script;
                    let userdata = node.userdata_of(ComponentId::new_unck(comp_index));
                    let script = engine.script_manager.script(script);

                    (
                        script.functions.clone(),
                        userdata,
                        script.path(),
                    )
                };


                functions.ready(path, &userdata);
                println!("node {:?} idx {comp_index}", node);
            }
        }


        *scene.root()
            .map(|x| hashmap.get(&x).unwrap())
            .unwrap()
    }


    pub fn len(&self) -> usize { self.map.inner_unck().len() }


    pub fn get(&self, handle: NodeId) -> &Node {
        self.map.get(handle.0).unwrap()
    }


    pub fn get_mut(&mut self, handle: NodeId) -> &mut Node {
        self.map.get_mut(handle.0).unwrap()
    }


    pub fn set_global_position(&mut self, of: NodeId, mut pos: Vec2) {
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
        let node = self.get(of);
        let mut target_parent = node.parent;

        while let Some(parent) = target_parent {
            let this = self.get(parent);

            rot -= this.properties.rotation;

            target_parent = this.parent;
        }

        self.get_mut(of).properties.rotation = rot;
    }


    pub fn iter_vec(&self) -> Vec<NodeId> {
        let mut stack = vec![];
        let mut coll = vec![];

        if let Some(root) = self.root() {
            stack.push(root);
        }


        while let Some(node) = stack.pop() {
            coll.push(node);

            let node = self.get(node);
            stack.extend_from_slice(&node.children);
        }

        coll.reverse();

        coll
    }


    pub fn root(&self) -> Option<NodeId> {
        if self.len() >= 1 { Some(NodeId(Handle { gen: 0, idx: 0 })) }
        else { None }
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


impl SceneTree {
    pub fn to_string(&self, script_manager: &ScriptManager) -> String {
        let this = self.trim();
        
        let mut nodes_file = toml::Table::new();

        for (i, node) in this.map.iter().enumerate().skip(1) {
            let node = this.map.get(node).unwrap();
            let mut table = toml::Table::new();

            table.insert("position.x".to_string(), node.properties.position.x.into());
            table.insert("position.y".to_string(), node.properties.position.y.into());

            let mut comps = toml::Table::new();
            for (id, comp) in node.components.iter() {
                if comp.script != ScriptId::EMPTY {
                    let name = script_manager.script(comp.script).path();
                    comps.insert(id.usize().to_string(), toml::Value::String(name.to_string()));
                }
            }

            table.insert("components".to_string(), toml::Value::Table(comps));

            if let Some(parent) = &node.parent {
                table.insert("parent".to_string(), toml::Value::Integer(parent.0.idx as i64));
            }


            nodes_file.insert(i.to_string(), toml::Value::Table(table));
        }


        toml::to_string_pretty(&nodes_file).unwrap()
    }
}


impl NodeId {
    /// Creates a new `NodeId` with the generation of it being 0
    pub fn from_idx(idx: u32) -> Self {
        Self(Handle { gen: 0, idx: idx as usize })
    }
    pub fn idx(&self) -> usize { self.0.idx }
}
