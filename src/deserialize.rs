use std::str::FromStr;

use sti::keyed::KVec;
use tracing::{error, info, trace, warn, Level};

use crate::{engine::Engine, math::vector::Vec3, scene_manager::{node::NodeProperties, scene_template::{TemplateComponent, TemplateComponents, TemplateNode, TemplateNodeId, TemplateScene}}, script_manager::{fields::{FieldType, FieldValue}, ScriptManager}};

impl TemplateScene {
    /// Loads a file as a 'TemplateScene'
    /// Returns an empty 'TemplateScene' if an error occurs
    pub fn from_file<A>(engine: &mut Engine, path: A) -> TemplateScene
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
            return TemplateScene::new();
        };

        let toml_table = match toml::Table::from_str(&scene_data) {
            Ok(v) => v,
            Err(e) => {
                error!("unable to parse the file as a toml table: {e}");
                return TemplateScene::new();
            }
        };

        TemplateScene::from_table(engine, &toml_table)
    }


    /// Loads a file as a 'TemplateScene'
    /// Returns an empty 'TemplateScene' if an error occurs
    pub fn from_table(engine: &mut Engine, table: &toml::Table) -> TemplateScene {
        let mut scene = TemplateScene::new();
        let inner_scene = scene.inner_mut();
        
        // saved scenes must be trimmed down to their bare minimums
        // this provides us an easy way to check out of bounds nodes
        let expected_scene_size = table.len() as u32;

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

            let node = TemplateNode::from_entry(engine, value);
            let Some(node) = node
            else {
                has_errored = true;
                continue;
            };


            if let Some(parent) = node.parent {
                if parent.inner() >= expected_scene_size {
                    error!("the parent '{}' is out of bounds", parent.inner());
                    has_errored = true;
                    continue;
                }


                if parent.inner() as u32 == node_index {
                    error!("a node can't be a parent of itself");
                    has_errored = true;
                    continue;
                }
            }


            if inner_scene.len() > node_index as usize {
                inner_scene.insert(node_index as usize, node);
            } else {
                inner_scene.push(node);
            }
        }


        // better to return nothing than to return corrupt
        if has_errored {
            return TemplateScene::new();
        }


        // not even a root.
        if table.len() == 0 {
            warn!("no root node provided (index = 0)");
            return TemplateScene::new();
        }


        // we do `largest_node_index + 1` as node indexes start from 0
        if inner_scene.len() != largest_node_index as usize + 1 {
            error!("the scene file must be compacted down. the largest index \
                   is '{}' while the item count is '{}'",
                   largest_node_index + 1, scene.len());
            return TemplateScene::new();
        }


        scene
    }
}


impl TemplateNode {
    pub fn from_entry(engine: &mut Engine, value: &toml::Value) -> Option<Self> {
        let Some(table) = value.as_table()
        else {
            error!("can't parse the value as a table");
            return None;
        };

        let properties = NodeProperties::from_table(engine, &table);

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
                break 'me Some(TemplateComponents::new(KVec::new()));
            };

            let Some(components) = components.as_table()
            else {
                error!("the components entry exists but it's not a table");
                break 'me None;
            };

            TemplateComponents::from_table(engine, components)
        };


        Some(TemplateNode {
            properties: properties?,
            parent: parent.map(|x| TemplateNodeId::new_unck(x)),
            components: components?,
        })
    }
}



impl TemplateComponents {
    pub fn from_table(engine: &mut Engine, table: &toml::Table) -> Option<Self> {
        let mut vec = KVec::with_cap(table.len());
        let mut has_errored = false;

        for (name, fields) in table.iter() {
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

            let script_id = ScriptManager::load_script(engine, &name);
            let script_manager = &engine.get().script_manager;
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

            vec.push(TemplateComponent::new(
                    script_id,
                    fields,
            ));
        }


        if has_errored { return None }

        Some(TemplateComponents::new(vec))
    }
}



impl FieldValue {
    pub fn from_toml(value: &toml::Value, suggestion: &FieldType) -> Option<Self> {
        Some(match value {
            toml::Value::String(v) => Self::String(Engine::lua().create_string(v).unwrap()),
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
