use std::collections::HashMap;

use sti::{define_key, keyed::KVec};
use tracing::{info, info_span};

use crate::{engine::Engine, scene_manager::node::{Components, Node}, script_manager::{fields::{FieldId, FieldValue}, ScriptId}};

use super::{node::{Component, ComponentId, NodeProperties}, NodeId, SceneManager, TemplateId};


define_key!(u32, pub TemplateNodeId);
define_key!(u32, pub TemplateComponentId);

#[derive(Debug)]
pub struct TemplateScene {
    nodes: KVec<TemplateNodeId, TemplateNode>,
}


#[derive(Debug)]
pub struct TemplateNode {
    pub properties: NodeProperties,
    pub parent: Option<TemplateNodeId>,
    pub components: TemplateComponents,
}


#[derive(Debug)]
pub struct TemplateComponents {
    map: KVec<TemplateComponentId, TemplateComponent>,
}


#[derive(Debug)]
pub struct TemplateComponent {
    script: ScriptId,
    fields: KVec<FieldId, FieldValue>,
}


impl TemplateScene {
    pub fn new() -> Self {
        Self { nodes: KVec::new() }
    }


    pub fn len(&self) -> usize {
        self.nodes.len()
    }


    pub fn inner_mut(&mut self) -> &mut sti::prelude::Vec<TemplateNode> {
        self.nodes.inner_mut_unck()
    }


    pub fn instantiate(engine: &mut Engine, template_id: TemplateId) -> Option<NodeId> {
        info!("instantiating template scene {template_id:?}");
        let mut hashmap = HashMap::new();
        let mut engine_ref = engine.get_mut();
        let sm = &mut engine_ref.scene_manager;

        let this = &sm.templates[template_id];

        let mut root = None;
        for (node_id, template_node) in this.nodes.iter() {
            if root == None {
                root = Some(node_id);
            }
            
            let components = {
                let mut vec = KVec::with_cap(template_node.components.map.len());

                for (i, comp) in template_node.components.map.iter().enumerate() {
                    let comp_id = ComponentId::new_unck(i as u32);
                    vec.push(Component::new(comp_id,
                                            comp.1.script,
                                            comp.1.fields.clone()));
                }

                Components::new(vec)
            };


            let insert_node = Node {
                node_id: NodeId::PLACEHOLDER,
                properties: template_node.properties,
                children: vec![],
                parent: None,
                components,
                userdata: None,
                queued_free: false,
            };

            let insert_id = sm.tree.insert(insert_node);

            hashmap.insert(node_id, insert_id);

            if let Some(parent) = template_node.parent {
                // this will NEVER return 'None', this is because
                // this loop is stack based and the parent will be
                // inserted into the hashmap before the children
                let parent_id = hashmap.get(&parent).unwrap();
                sm.tree.set_parent(insert_id, Some(*parent_id));
            }


            let insert_node = sm.tree.get_mut(insert_id);
            insert_node.node_id = insert_id;
        }


        drop(engine_ref);

        let root = root?;
        let root = *hashmap.get(&root).unwrap();
        SceneManager::call_ready(engine, root);

        Some(root)
    }
}


impl TemplateComponents {
    pub fn new(map: KVec<TemplateComponentId, TemplateComponent>) -> Self {
        Self { map }
    }
}


impl TemplateComponent {
    pub fn new(script: ScriptId, fields: KVec<FieldId, FieldValue>) -> Self {
        Self { script, fields }
    }
}
