use mlua::{AnyUserData, Error, FromLua, Value, Vector};
use rapier2d::math::Rotation;
use tracing::info;

use crate::{engine::Engine, math::vector::Vec3, scene_manager::{node::ComponentId, scene_tree::NodeId}, script_manager::fields::{FieldType, FieldValue}};

#[derive(Debug, Clone, Copy)]
pub struct NodeUserData(pub NodeId, pub ComponentId);


impl<'a> mlua::UserData for NodeUserData {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("position", |_, NodeUserData(this, _)| Ok(Engine::get().scene_manager.borrow().current.get(*this).borrow().properties.position));
        fields.add_field_method_get("scale", |_, NodeUserData(this, _)| Ok(Engine::get().scene_manager.borrow().current.get(*this).borrow().properties.scale));
        fields.add_field_method_get("rotation", |_, NodeUserData(this, _)| Ok(Engine::get().scene_manager.borrow().current.get(*this).borrow().properties.rotation));

        fields.add_field_method_set("position", |_, NodeUserData(this, _), ass| {
            let engine = Engine::get();
            let mut scene = engine.scene_manager.borrow_mut();
            let mut node = scene.current.get(*this).borrow_mut();
            node.properties.position = ass;
            drop(node);

            let mut stack = vec![];
            stack.push(*this);

            while let Some(this) = stack.pop() {
                let node = scene.current.get(this).borrow();
                stack.extend_from_slice(&node.children);

                let Some(rb) = scene.physics.node_to_rigidbody.get(&this).copied()
                else { continue };

                let pos = node.global_position(&scene.current);
                drop(node);
                scene.physics.get_rb_mut(rb).set_position(rapier2d::na::Vector2::new(pos.x, pos.y).into(), true);
            }

            Ok(())
        });

        fields.add_field_method_set("rotation", |_, NodeUserData(this, _), ass: f32| {
            let engine = Engine::get();
            let mut scene = engine.scene_manager.borrow_mut();
            let mut node = scene.current.get(*this).borrow_mut();
            node.properties.rotation = ass;
            drop(node);

            let mut stack = vec![];
            stack.push(*this);

            while let Some(this) = stack.pop() {
                let node = scene.current.get(this).borrow();
                stack.extend_from_slice(&node.children);

                let Some(rb) = scene.physics.node_to_rigidbody.get(&this).copied()
                else { continue };

                let rot = node.global_rotation(&scene.current);
                drop(node);
                scene.physics.get_rb_mut(rb).set_rotation(Rotation::from_angle(rot), true);
            }

            Ok(())
        });


        fields.add_field_method_set("scale", |_, NodeUserData(this, _), ass| Ok(Engine::get().scene_manager.borrow().current.get(*this).borrow_mut().properties.scale = ass));
        fields.add_field_method_set("sprite", |_, NodeUserData(this, _), ass| Ok(Engine::get().scene_manager.borrow().current.get(*this).borrow_mut().properties.texture = ass));


        fields.add_field_method_get("global_position", |_, NodeUserData(this, _)| 
            Ok(Engine::get().scene_manager.borrow().current.get(*this).borrow().global_position(&Engine::get().scene_manager.borrow().current))
        );


        fields.add_field_method_get("global_scale", |_, NodeUserData(this, _)| 
            Ok(Engine::get().scene_manager.borrow().current.get(*this).borrow().global_scale(&Engine::get().scene_manager.borrow().current))
        );

        fields.add_field_method_get("parent", |_, this| {
            let scene = Engine::get().scene_manager.borrow();
            let node = scene.current.get(this.0).borrow();
            let parent = node.parent;

            Ok(match parent {
                Some(v) => Value::UserData(scene.current.get(v).borrow().userdata.clone()),
                None => Value::Nil,
            })
        });

        fields.add_field_method_set("parent", |_, this, parent: Option<NodeId>| {
            let scene = Engine::get().scene_manager.borrow();
            scene.current.set_parent(this.0, parent);
            Ok(())
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__index", |_, NodeUserData(node, comp), name: String| {
            let scene = Engine::get().scene_manager.borrow();
            let script_manager = Engine::get().script_manager.borrow();

            let node = scene.current.get(*node).borrow();
            let comp = node.components.get(*comp);
            
            let script = script_manager.script(comp.script);

            let Some(field) = script.fields_ids.get(&name)
            else { return Err(Error::RuntimeError(format!("field '{}' doesn't exist", name))) };

            let field = &comp.fields[*field];

            let val = match field {
                FieldValue::Float(v) => mlua::Value::Number(*v),
                FieldValue::Integer(v) => mlua::Value::Integer(*v),
                FieldValue::Vec3(vec3) => mlua::Value::Vector(Vector::new(vec3.x, vec3.y, vec3.z)),
                FieldValue::Table(table) => mlua::Value::Table(table.clone()),
                FieldValue::Script(Some(any_user_data)) => mlua::Value::UserData(any_user_data.clone()),
                FieldValue::Script(None) => mlua::Value::Nil,
                FieldValue::Any(value) => value.clone(),
                FieldValue::Bool(v) => mlua::Value::Boolean(*v),
                FieldValue::String(v) => mlua::Value::String(mlua::String::from(v.clone())),
            };

            Ok(val)
        });

        methods.add_meta_method("__newindex", |lua, NodeUserData(node, comp), (name, value): (String, mlua::Value)| {
            let (field_id, field) = {
                let scene = Engine::get().scene_manager.borrow_mut();
                let mut node = scene.current.get(*node).borrow_mut();
                let comp = node.components.get_mut(*comp);
                
                let script_manager = Engine::get().script_manager.borrow();
                let script = script_manager.script(comp.script);

                let Some(field) = script.fields_ids.get(&name)
                else { return Err(Error::RuntimeError(format!("field '{}' doesn't exist", name))) };

                (*field, script.fields_vec[*field].ty)
            };

            let field = match field {
                FieldType::Float => FieldValue::Float(f64::from_lua(value, lua)?),
                FieldType::Integer => FieldValue::Integer(i32::from_lua(value, lua)?),
                FieldType::Bool => FieldValue::Bool(bool::from_lua(value, lua)?),
                FieldType::String => FieldValue::String(mlua::String::from_lua(value, lua)?),
                FieldType::Vec3 => FieldValue::Vec3(Vec3::from_lua(value, lua)?),
                FieldType::AnyTable => FieldValue::Table(mlua::Table::from_lua(value, lua)?),
                FieldType::Script(_) => {
                    // @TODO: CHECK THE TYPE OF THE SCRIPT
                    if !value.is_nil() { FieldValue::Script(Some(AnyUserData::from_lua(value, lua)?)) }
                    else { FieldValue::Script(None) }
                },
                FieldType::Any => FieldValue::Any(value),
            };

            {
                let scene = Engine::get().scene_manager.borrow_mut();
                let mut node = scene.current.get(*node).borrow_mut();
                let comp = node.components.get_mut(*comp);

                comp.fields[field_id] = field;
            }

            Ok(())
        });

        methods.add_method("get_component", |_, this, name: String| {
            let comp = 'b: {
                let scene = Engine::get().scene_manager.borrow();
                let script_manager = Engine::get().script_manager.borrow();
                let mut node = scene.current.get(this.0).borrow_mut();

                for c in node.components.iter_mut() {
                    let script = c.1.script;
                    let script = script_manager.script(script);

                    if script.name == name {
                        let val = if !c.1.is_ready {
                                info!("get_component: '{}' wasn't ready", script.name);
                                c.1.is_ready = true;
                                Some((script.functions.clone(), script.path()))
                            } else { None };
                        break 'b (c.1.userdata.clone(), val)
                    }
                }

                return Ok(Value::Nil)
            };

            if let Some(script) = comp.1 {
                script.0.ready(script.1, &comp.0);
            }

            Ok(Value::UserData(comp.0))
        });

        methods.add_method("get_child", |_, this, idx: usize| {
            let scene = Engine::get().scene_manager.borrow();
            let node = scene.current.get(this.0).borrow();

            let target = node.children[idx];
            let target = scene.current.get(target).borrow().userdata.clone();
            Ok(target)
        });
    }

}


impl mlua::UserData for NodeId {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_component", |_, this, name: String| {
            let comp = 'b: {
                let scene = Engine::get().scene_manager.borrow_mut();
                let script_manager = Engine::get().script_manager.borrow();
                let mut node = scene.current.get(*this).borrow_mut();

                for c in node.components.iter_mut() {
                    let script = c.1.script;
                    let script = script_manager.script(script);

                    if script.name == name {
                        let val = if !c.1.is_ready {
                                info!("get_component: '{}' wasn't ready", script.name);
                                c.1.is_ready = true;
                                Some((script.functions.clone(), script.path()))
                            } else { None };
                        break 'b (c.1.userdata.clone(), val)
                    }
                }

                return Ok(Value::Nil)
            };

            if let Some(script) = comp.1 {
                script.0.ready(script.1, &comp.0);
            }

            Ok(Value::UserData(comp.0))
        });

    }
}



impl FromLua for NodeId {
    fn from_lua(value: Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let value = value.as_userdata().map(|x| x.borrow::<Self>().ok()).flatten();
        let Some(value) = value
        else {
            return Err(Error::runtime(format!("expected a 'NodeId' found '{:?}'", value)));
        };

        Ok(*value)
    }
}


impl FromLua for NodeUserData {
    fn from_lua(value: Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let value = value.as_userdata().map(|x| x.borrow::<Self>().ok()).flatten();
        let Some(value) = value
        else {
            return Err(Error::runtime(format!("expected a 'NodeId' found '{:?}'", value)));
        };

        Ok(*value)
    }
}
