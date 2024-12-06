use std::{cell::RefCell, collections::HashMap};

use genmap::{GenMap, Handle};
use sti::{define_key, keyed::Key};
use toml::to_string_pretty;

use crate::{engine::Engine, script_manager::{ScriptId, ScriptManager}};

use super::node::Node;

define_key!(u32, pub NodeIndex);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(Handle);

#[derive(Clone, Debug)]
pub struct SceneTree {
    pub map: GenMap<RefCell<Node>>,
}


impl SceneTree {
    pub fn new() -> Self {
        Self { map: GenMap::with_capacity(0) }
    }


    pub fn insert(&mut self, node: Node) -> NodeId {
        NodeId(self.map.insert(RefCell::new(node)))
    }


    pub fn trim(&self) -> SceneTree {
        let mut new = SceneTree::new();
        let mut hm = HashMap::new();

        for handle in self.map.iter() {
            let node = self.map.get(handle).unwrap();
            let new_handle = new.insert(node.clone().into_inner());
            hm.insert(NodeId(handle), new_handle);
        }

        for new_handle in hm.values() {
            let new_handle = hm.get(new_handle).unwrap();
            let mut new_node = new.map.get_mut(new_handle.0).unwrap().borrow_mut();

            for c in new_node.children.iter_mut() {
                *c = *hm.get(c).unwrap();
            }
        }

        new
    }


    pub fn instantiate(&mut self, scene: &SceneTree) -> NodeId {
        let mut hashmap = HashMap::new();

        let mut stack = vec![];
        if let Some(root) = scene.root() {
            stack.push(root);
        }


        while let Some(node_id) = stack.pop() {
            let node = scene.get(node_id).borrow();

            let insert_node = Node {
                properties: node.properties,
                children: vec![],
                parent: None,
                components: node.components.clone(),
                userdata: node.userdata.clone(), // placeholder
            };


            let insert_id = self.map.insert(insert_node.into());
            let insert_node_id = NodeId(insert_id);
            hashmap.insert(node_id, insert_node_id);

            if let Some(parent) = node.parent {
                self.set_parent(insert_node_id, Some(*hashmap.get(&parent).unwrap()));
            }

            self.get(insert_node_id).borrow_mut().userdata = Engine::get().lua.create_userdata(insert_node_id).unwrap();

            stack.extend_from_slice(&node.children);
        }


        *scene.root()
            .map(|x| hashmap.get(&x).unwrap())
            .unwrap()
    }


    pub fn len(&self) -> usize { self.map.inner_unck().len() }


    pub fn get(&self, handle: NodeId) -> &RefCell<Node> {
        let rc = self.map.get(handle.0).unwrap();
        rc
    }


    pub fn iter_vec(&self) -> Vec<NodeId> {
        let mut stack = vec![];
        let mut coll = vec![];

        if let Some(root) = self.root() {
            stack.push(root);
        }


        while let Some(node) = stack.pop() {
            coll.push(node);

            let node = self.get(node).borrow();
            stack.extend_from_slice(&node.children);
        }

        coll.reverse();

        coll
    }


    pub fn root(&self) -> Option<NodeId> {
        if self.len() >= 1 { Some(NodeId(Handle { gen: 0, idx: 0 })) }
        else { None }
    }


    pub fn set_parent(&self, of: NodeId, to: Option<NodeId>) {
        let mut of_node = self.get(of).borrow_mut();
        let old_parent_id = of_node.parent;

        of_node.parent = to;

        if let Some(old_parent_id) = old_parent_id {
            let mut old_parent = self.get(old_parent_id).borrow_mut();
            let (index, _) = old_parent.children.iter()
                .enumerate()
                .find(|x| *x.1 == of)
                .unwrap();

            old_parent.children.remove(index);
        }


        if let Some(to) = to {
            let mut to_parent = self.get(to).borrow_mut();
            to_parent.children.push(of);
        }
    }
}


impl SceneTree {
    pub fn to_string(&self, script_manager: &ScriptManager) -> String {
        let this = self.trim();
        
        let mut nodes_file = toml::Table::new();

        for (i, node) in this.map.iter().enumerate().skip(1) {
            let node = this.map.get(node).unwrap().borrow();
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
