use std::{cell::RefCell, str::FromStr};

use sti::keyed::{KVec, Key};
use tracing::{error, info, trace, warn, Level};

use crate::{engine::Engine, lua::node::NodeUserData, math::vector::Vec3, scene_manager::{node::{Component, ComponentId, Components, Node, NodeProperties}, scene_tree::{NodeId, SceneTree}}, script_manager::fields::{FieldType, FieldValue}};

impl SceneTree {
    /// Loads a file as a 'SceneTree'
    /// Returns an empty 'SceneTree' if an error occurs
    pub fn from_file<A>(path: A) -> SceneTree
    where A: AsRef<std::path::Path> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();

        let span = tracing::span!(Level::ERROR, "deserialize ", 
                                  path = &*path_str);
        let _handle = span.entered();

        info!("reading scene '{}'", path_str);


        let Ok(scene_data) = std::fs::read_to_string(path)
        else {
            error!("unable to read");
            return SceneTree::new();
        };

        let toml_table = match toml::Table::from_str(&scene_data) {
            Ok(v) => v,
            Err(e) => {
                error!("unable to parse the file as a toml table: {e}");
                return SceneTree::new();
            }
        };

        SceneTree::from_table(&toml_table)
    }


    /// Loads a file as a 'SceneTree'
    /// Returns an empty 'SceneTree' if an error occurs
    pub fn from_table(table: &toml::Table) -> SceneTree {
        let mut scene = SceneTree::new();
        let inner_scene = scene.map.inner_unck_mut();
        
        // saved scenes must be trimmed down to their bare minimums
        // this provides us an easy way to check out of bounds nodes
        let expected_scene_size = table.len();

        let mut largest_node_index = 0;
        let mut has_errored = false; 


        for (entry, value) in table {
            let span = tracing::span!(Level::ERROR, "", node = entry);
            let _handle = span.entered();

            let node_index = match entry.parse::<u32>() {
                Ok(v) => v,
                Err(e) => {
                    error!("failed to parse the name as a node index: {e}");
                    has_errored = true;
                    continue;
                },
            };

            largest_node_index = largest_node_index.max(node_index);

            let node = Node::from_entry(NodeId::from_idx(node_index), value);
            let Some(node) = node
            else {
                has_errored = true;
                continue;
            };


            if let Some(parent) = node.parent {
                if parent.idx() >= expected_scene_size {
                    error!("the parent '{}' is out of bounds", parent.idx());
                    has_errored = true;
                    continue;
                }


                if parent.idx() as u32 == node_index {
                    error!("a node can't be a parent of itself");
                    has_errored = true;
                    continue;
                }
            }


            let entry = RefCell::new(node);
            let entry = genmap::Slot::Occupied { itm: entry };
            let entry = (0, entry);

            if inner_scene.len() > node_index as usize {
                inner_scene.insert(node_index as usize, entry);
            } else {
                inner_scene.push(entry);
            }
        }


        // better to return nothing than to return corrupt
        if has_errored {
            return SceneTree::new();
        }


        // not even a root.
        if table.len() == 0 {
            warn!("no root node provided (index = 0)");
            return SceneTree::new();
        }


        // we do `largest_node_index + 1` as node indexes start from 0
        if inner_scene.len() != largest_node_index as usize + 1 {
            error!("the scene file must be compacted down. the largest index \
                   is '{}' while the item count is '{}'",
                   largest_node_index + 1, scene.len());
            return SceneTree::new();
        }


        // initialize the 'Node.children' vec
        let mut index = 0;
        while index < inner_scene.len() {
            index += 1;

            let genmap::Slot::Occupied { itm } = &inner_scene[index-1].1
            else { unreachable!() };

            let Some(parent) = itm.borrow().parent
            else { continue };

            let genmap::Slot::Occupied { itm } = &inner_scene[parent.idx()].1
            else { unreachable!() };

            itm.borrow_mut().children.push(NodeId::from_idx(index as u32 - 1));
        }

        scene
    }
}


impl Node {
    pub fn from_entry(index: NodeId, value: &toml::Value) -> Option<Node> {
        let Some(table) = value.as_table()
        else {
            error!("can't parse the value as a table");
            return None;
        };

        let properties = NodeProperties::from_table(&table);

        let parent = 'me: {
            let Some(parent) = table.get("parent")
            else { break 'me None };

            let Some(node_id) = parent.as_integer()
            else {
                error!("failed to parse the parent as \
                       a node index: not an integer");
                break 'me None;
            };


            let node_id : u32 = match node_id.try_into() {
                Ok(v) => v,
                Err(e) => {
                    error!("failed to parse the parent as a node index: {e}");
                    break 'me None;
                },
            };

            Some(node_id)
        };


        let components = 'me: {
            let Some(components) = table.get("components")
            else {
                break 'me Some(Components::empty());
            };

            let Some(components) = components.as_table()
            else {
                error!("the components entry exists but it's not a table");
                break 'me None;
            };

            Components::from_table(index, components)
        };


        Some(Node {
            properties: properties?,
            children: vec![], // this field must be filled later on
            parent: parent.map(NodeId::from_idx),
            components: components?,
            userdata: Engine::get().lua.create_userdata(index).unwrap(),
        })
    }
}



impl Components {
    pub fn from_table(node: NodeId, table: &toml::Table) -> Option<Components> {
        let mut script_manager = Engine::get().script_manager.borrow_mut();

        let mut vec = KVec::with_cap(table.len());
        let mut has_errored = false;

        for (index, (name, fields)) in table.iter().enumerate() {
            let span = tracing::span!(Level::ERROR, "", component = name);
            let _handle = span.entered();

            let Some(fields_table) = fields.as_table()
            else {
                error!("the field value of the component \
                       '{name}' isn't a table");
                has_errored = true;
                continue;
            };

            let fields_table = fields_table.clone();

            let script_id = script_manager.load_script(&name);
            let script = script_manager.script(script_id);
            let mut fields = KVec::with_cap(script.fields_vec.len());

            for field in script.fields_vec.iter() {
                let span = tracing::span!(Level::TRACE, "", field = field.1.name);
                let _handle = span.entered();

                let name = &field.1.name;
                let Some(value) = fields_table.get(name)
                else {
                    trace!("the field '{}' isn't specified, \
                           using the default value", field.1.name);
                    fields.push(field.1.value.clone());
                    continue
                };

                let field_value = FieldValue::from_toml(value, &field.1.ty);
                let Some(field_value) = field_value
                else {
                    error!("failed to parse the value of '{}',
                           '{}' is an unsupported type of toml value",
                           field.1.name, value.type_str());
                    fields.push(field.1.value.clone());
                    continue;
                };

                if field.1.ty != field_value.ty() {
                    error!("the type of field must be '{:?}' but \
                           the value provided is of type '{:?}'",
                           field.1.ty, field_value.ty());
                    has_errored = true;
                    fields.push(field.1.value.clone());
                    continue;
                }

                fields.push(field_value);
            }

            let comp_id = ComponentId::from_usize(index).unwrap();
            let userdata = NodeUserData(node, comp_id);

            vec.push(Component::new(
                    script_id,
                    fields,
                    Engine::get().lua.create_userdata(userdata).unwrap(),
            ));
        }


        if has_errored { return None }

        Some(Components::new(vec))
    }
}



impl FieldValue {
    pub fn from_toml(value: &toml::Value, suggestion: &FieldType) -> Option<Self> {
        Some(match value {
            toml::Value::String(v) => Self::String(Engine::get().lua.create_string(v).unwrap()),
            toml::Value::Integer(v) => Self::Integer(*v as i32),
            toml::Value::Float(v) => Self::Float(*v),
            toml::Value::Boolean(v) => Self::Bool(*v),

            toml::Value::Datetime(_) => return None,
            toml::Value::Array(_) => return None,

            toml::Value::Table(map) => {
                if suggestion == &FieldType::Vec3 {
                    let x = map.get("x")
                        .map(|x| x.as_float())
                        .unwrap_or(Some(0.0))
                        .unwrap_or_else(|| {
                            error!("failed to read 'x' as a float");
                            0.0
                        });

                    let y = map.get("y")
                        .map(|x| x.as_float())
                        .unwrap_or(Some(0.0))
                        .unwrap_or_else(|| {
                            error!("failed to read 'y' as a float");
                            0.0
                        });

                    let z = map.get("z")
                        .map(|x| x.as_float())
                        .unwrap_or(Some(0.0))
                        .unwrap_or_else(|| {
                            error!("failed to read 'z' as a float");
                            0.0
                        });

                    let vec = Vec3::new(x as f32, y as f32, z as f32);
                    return Some(Self::Vec3(vec))
                };

                todo!()
            },
        })
    }


    pub fn ty(&self) -> FieldType {
        match self {
            FieldValue::Float(_) => FieldType::Float,
            FieldValue::Integer(_) => FieldType::Integer,
            FieldValue::Vec3(_) => FieldType::Vec3,
            FieldValue::Table(_) => FieldType::AnyTable,
            FieldValue::Script(_) => todo!(),
            FieldValue::Any(_) => FieldType::Any,
            FieldValue::Bool(_) => FieldType::Bool,
            FieldValue::String(_) => FieldType::String,
        }
    }
}
