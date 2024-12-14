use mlua::AnyUserData;
use sti::{define_key, keyed::{KIterMut, KVec}};
use tracing::error;

use crate::{asset_manager::{texture::TextureLoadType, AssetManager, TextureId}, engine::Engine, lua::node::NodeUserData, math::vector::{Colour, Vec2, Vec4}, script_manager::{fields::{FieldId, FieldValue}, ScriptId}};

use super::{NodeId, scene_tree::SceneTree};

define_key!(u32, pub ComponentId);

#[derive(Debug, Clone)]
pub struct Node {
    pub node_id: NodeId,
    pub properties: NodeProperties,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
    pub components: Components,
    pub queued_free: bool,
    pub userdata: Option<AnyUserData>,
}


#[derive(Debug, Clone, Copy)]
pub struct NodeProperties {
    pub position: Vec2,
    pub modulate: Colour,
    pub scale: Vec2,
    pub rotation: f32,
    pub texture: Option<TextureId>,
}


#[derive(Debug, Clone)]
pub struct Components {
    vec: KVec<ComponentId, Component>
}


#[derive(Debug, Clone)]
pub struct Component {
    pub comp_id: ComponentId,
    pub script: ScriptId,
    pub fields: KVec<FieldId, FieldValue>,
    pub is_ready: bool,
    userdata: Option<AnyUserData>,
}


pub struct ComponentIter {
    curr: u32,
    max: u32,
}


impl NodeProperties {
    pub fn new(position: Vec2, modulate: Colour, scale: Vec2, rotation: f32, texture: Option<TextureId>) -> Self {
        Self { position, modulate, scale, rotation, texture }
    }
}


impl Node {
    pub fn get_comp(&self, comp: ComponentId) -> &Component {
        &self.components.vec[comp]
    }


    pub fn get_comp_mut(&mut self, comp: ComponentId) -> &mut Component {
        &mut self.components.vec[comp]
    }


    pub fn global_position(&self, nodes: &SceneTree) -> Vec2 {
        let mut target_parent = self.parent;
        let mut pos = self.properties.position;

        while let Some(parent) = target_parent {
            let this = nodes.get(parent);

            pos.x *= this.properties.scale.x;
            pos.y *= this.properties.scale.y;
            pos.x += this.properties.position.x;
            pos.y += this.properties.position.y;

            target_parent = this.parent;
        }

        pos
    }


    pub fn global_rotation(&self, nodes: &SceneTree) -> f32 {
        let mut target_parent = self.parent;
        let mut rot = self.properties.rotation;

        while let Some(parent) = target_parent {
            let this = nodes.get(parent);
            rot += this.properties.rotation;
            target_parent = this.parent;
        }

        rot
    }


    pub fn global_scale(&self, nodes: &SceneTree) -> Vec2 {
        let mut target_parent = self.parent;
        let mut scale = self.properties.scale;

        while let Some(parent) = target_parent {
            let this = nodes.get(parent);

            scale.x *= this.properties.scale.x;
            scale.y *= this.properties.scale.y;

            target_parent = this.parent;
        }

        scale
    }


    pub fn userdata(&mut self) -> AnyUserData {
        if let Some(userdata) = &self.userdata {
            return userdata.clone();
        }

        self.userdata = Some(Engine::lua().create_userdata(self.node_id).unwrap());
        self.userdata.as_ref().unwrap().clone()
    }


    pub fn userdata_of(&mut self, comp: ComponentId) -> AnyUserData {
        let comp = self.components.get_mut(comp);
        if let Some(userdata) = &comp.userdata {
            return userdata.clone();
        }

        comp.userdata = Some(Engine::lua().create_userdata(NodeUserData(self.node_id, comp.comp_id)).unwrap());
        comp.userdata.as_ref().unwrap().clone()
    }
}


impl Components {
    pub fn new(vec: KVec<ComponentId, Component>) -> Self {
        Self { vec }
    }


    pub fn empty() -> Self {
        Self::new(KVec::new())
    }


    pub fn get(&self, key: ComponentId) -> &Component {
        &self.vec[key]
    }


    pub fn get_mut(&mut self, key: ComponentId) -> &mut Component {
        &mut self.vec[key]
    }


    pub fn len(&self) -> usize {
        self.vec.len()
    }


    pub fn get_index(&self, index: u32) -> &Component {
        self.get(ComponentId(index))
    }


    pub fn get_mut_index(&mut self, index: u32) -> &mut Component {
        self.get_mut(ComponentId(index))
    }


    pub fn iter(&self) -> ComponentIter {
        ComponentIter { curr: 0, max: self.vec.len() as u32 }
    }


    pub fn iter_mut<'a>(&'a mut self) -> KIterMut<'a, ComponentId, Component> {
        self.vec.iter_mut()
    }
}


impl Component {
    pub fn new(comp_id: ComponentId, script: ScriptId, fields: KVec<FieldId, FieldValue>) -> Self {
        Self {
            script,
            fields,
            is_ready: false,
            userdata: None,
            comp_id,
        }
    }
}


impl NodeProperties {
    pub fn identity() -> Self {
        Self::new(Vec2::new(0.0, 0.0), Colour::new(1.0, 1.0, 1.0, 1.0), Vec2::new(1.0, 1.0), 0.0, None)
    }


    pub fn merge(mut self, oth: NodeProperties) -> Self {
        self.scale.x *= oth.scale.x;
        self.scale.y *= oth.scale.y;

        self.rotation += oth.rotation;
        self.modulate = self.modulate * oth.modulate;
        self.position.x *= oth.scale.x;
        self.position.y *= oth.scale.y;
        self.position.x += oth.position.x;
        self.position.y += oth.position.y;

        self
    }
}



impl NodeProperties {
    pub fn from_table(engine: &mut Engine,
                      table: &toml::Table) -> Option<Self> {
        let parent_name = "";
        fn read<T>(parent_name: &str, table: &toml::Table, property: &str,
                   f: impl FnOnce(&str, &toml::Table) -> Option<T>) -> Option<T> {

            let Some(table) = table.get(property)
            else { error!("failed to read '{property}' in '{parent_name}', property doesn't exist"); return None };

            let Some(table) = table.as_table()
            else { error!("failed to read '{property}' in '{parent_name}', property isn't a table"); return None };

            let Some(prop) = f(&parent_name, table)
            else { error!("failed to read '{property}' in '{parent_name}'"); return None };

            Some(prop)
        }

        let position = read(parent_name, table, "position", Vec2::from_table)?;
        let modulate = read(parent_name, table, "modulate", Vec4::from_table)?;
        let scale = read(parent_name, table, "scale", Vec2::from_table)?;

        let Some(rotation) = table.get("rotation")
        else { error!("failed to read 'rotation' in '{parent_name}', property doesn't exist"); return None };

        let Some(rotation) = rotation.as_float()
        else { error!("failed to read 'rotation' in '{parent_name}', property isn't a float"); return None };

        let rotation = rotation as f32;


        let texture = table.get("texture").map(|texture| {
            match texture.as_str() {
                Some(v) => Some(v),
                None => {
                    error!("failed to read 'texture' in '{parent_name}', texture must be a path string");
                    None
                },
            }
        }).flatten();

        let texture = texture.map(|texture| {
            let Some((ty, path)) = texture.split_once(':')
            else {
                error!("failed to read 'texture' in '{parent_name}', \
                       unable to parse the texture string, format must be '<type>:<path>'");
                return None;
            };

            match ty {
                "image" => engine.get_mut().asset_manager.from_image(path),
                "script" => AssetManager::from_script(engine, path),

                _ => {
                    error!("failed to read 'texture' in '{parent_name}', texture's type must be \
                           either 'image' or 'script' but it is '{ty}'");
                    None
                }
            }

        }).flatten();

        Some(Self {
            position,
            modulate,
            scale,
            rotation,
            texture,
        })
    }


    fn _to_table(self, asset_manager: &mut AssetManager) -> toml::Table {
        let mut table = toml::Table::new();
        table.insert("position".to_string(), self.position.to_table().into());
        table.insert("modulate".to_string(), self.modulate.to_table().into());
        table.insert("scale".to_string(), self.scale.to_table().into());
        table.insert("rotation".to_string(), self.rotation.into());
        if let Some(texture) = self.texture {
            let texture = asset_manager.texture(texture);
            let script = match texture.load_type() {
                TextureLoadType::Image(v) => format!("image:{v}"),
                TextureLoadType::Script(v) => format!("script:{v}"),
                TextureLoadType::Runtime => unreachable!(),
            };

            table.insert("texture".to_string(), script.into());
        }
        table
    }
}


impl Iterator for ComponentIter {
    type Item = ComponentId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr >= self.max { return None }

        self.curr += 1;
        Some(ComponentId::new_unck(self.curr-1))
    }
}
