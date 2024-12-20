use mlua::{AnyUserData, Error, FromLua, Value, Vector};
use rapier2d::{math::Rotation, na::Isometry2};
use tracing::info;

use crate::{engine::Engine, math::vector::Vec3, scene_manager::{node::ComponentId, NodeId}, script_manager::fields::FieldValue};

#[derive(Debug, Clone, Copy)]
pub struct NodeUserData(pub NodeId, pub ComponentId);


impl<'a> mlua::UserData for NodeUserData {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("position", |_, NodeUserData(this, _)| Ok(Engine::generate().get().scene_manager.tree.get(*this).properties.position));
        fields.add_field_method_get("scale", |_, NodeUserData(this, _)| Ok(Engine::generate().get().scene_manager.tree.get(*this).properties.scale));
        fields.add_field_method_get("rotation", |_, NodeUserData(this, _)| Ok(Engine::generate().get().scene_manager.tree.get(*this).properties.rotation));

        fields.add_field_method_set("position", |_, NodeUserData(this, _), ass| {
            let mut engine = Engine::generate();
            let mut engine = engine.get_mut();
            let scene = &mut engine.scene_manager;
            let node = scene.tree.get_mut(*this);
            node.properties.position = ass;

            let mut stack = vec![];
            stack.push(*this);

            while let Some(this) = stack.pop() {
                let node = scene.tree.get(this);
                stack.extend_from_slice(&node.children);

                let Some(rb) = scene.physics.node_to_rigidbody.get(&this).copied()
                else { continue };

                let pos = node.global_position(&scene.tree);
                let rot = node.global_rotation(&scene.tree);
                let iso = Isometry2::new(pos.into(), rot);
                scene.physics.get_rb_mut(rb).set_position(iso, true);
            }

            Ok(())
        });


        fields.add_field_method_set("rotation", |_, NodeUserData(this, _), ass| {
            let mut engine = Engine::generate();
            let mut engine = engine.get_mut();
            let scene = &mut engine.scene_manager;
            let node = scene.tree.get_mut(*this);
            node.properties.rotation = ass;

            let mut stack = vec![];
            stack.push(*this);

            while let Some(this) = stack.pop() {
                let node = scene.tree.get(this);
                stack.extend_from_slice(&node.children);

                let Some(rb) = scene.physics.node_to_rigidbody.get(&this).copied()
                else { continue };

                let rot = node.global_rotation(&scene.tree);
                scene.physics.get_rb_mut(rb).set_rotation(Rotation::from_angle(rot), true);
            }

            Ok(())
        });



        fields.add_field_method_set("scale", |_, NodeUserData(this, _), ass| Ok(Engine::generate().get_mut().scene_manager.tree.get_mut(*this).properties.scale = ass));
        fields.add_field_method_set("sprite", |_, NodeUserData(this, _), ass| Ok(Engine::generate().get_mut().scene_manager.tree.get_mut(*this).properties.texture = ass));
        fields.add_field_method_set("modulate", |_, NodeUserData(this, _), ass| Ok(Engine::generate().get_mut().scene_manager.tree.get_mut(*this).properties.modulate = ass));


        fields.add_field_method_get("global_position", |_, NodeUserData(this, _)| {
            let engine = Engine::generate();
            let engine = engine.get();
            Ok(engine.scene_manager.tree.get(*this).global_position(&engine.scene_manager.tree))
        });


        fields.add_field_method_get("global_scale", |_, NodeUserData(this, _)| {
            let engine = Engine::generate();
            let engine = engine.get();
            Ok(engine.scene_manager.tree.get(*this).global_scale(&engine.scene_manager.tree))
        });

        fields.add_field_method_get("parent", |_, this| {
            let mut engine = Engine::generate();
            let mut engine = engine.get_mut();
            let node = engine.scene_manager.tree.get(this.0);
            let parent = node.parent;

            Ok(match parent {
                Some(v) => Value::UserData(engine.scene_manager.tree
                                           .get_mut(v).userdata()),
                None => Value::Nil,
            })
        });

        fields.add_field_method_set("parent", |_, this, parent: Option<NodeId>| {
            Engine::generate().get_mut().scene_manager.tree.set_parent(this.0, parent);
            Ok(())
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__index", |_, NodeUserData(node, comp), name: String| {
            let engine = Engine::generate();
            let engine = engine.get();

            let node = engine.scene_manager.tree.get(*node);
            let comp = node.components.get(*comp);
            
            let script = engine.script_manager.script(comp.script);

            let Some(field) = script.fields.get(&name)
            else { return Err(Error::RuntimeError(format!("field '{}' doesn't exist", name))) };

            let field = &comp.fields[*field];
            Ok(field.value().clone())
        });


        methods.add_meta_method("__newindex", |lua, NodeUserData(node, comp), (name, value): (String, mlua::Value)| {
            Engine::generate().with(|engine| {
                let node = engine.scene_manager.tree.get_mut(*node);
                let comp = node.components.get_mut(*comp);
                
                let script = engine.script_manager.script(comp.script);

                let Some(field) = script.fields.get(&name)
                else { return Err(Error::RuntimeError(format!("eigj field '{}' doesn't exist", name))) };

                comp.fields[*field] = FieldValue::new(value);
                Ok(())
            })?;

            Ok(())
        });

        methods.add_method("get_component", |_, this, name: String| {
            let comp = 'b: {
                let mut engine = Engine::generate();
                let mut engine = engine.get_mut();
                let engine = &mut *engine;
                let node = engine.scene_manager.tree.get_mut(this.0);

                let mut comp_index = 0u32;
                loop {
                    comp_index += 1;
                    let comp_index = comp_index - 1;
                    
                    if comp_index as usize >= node.components.len() {
                        break
                    }

                    let comp_index = ComponentId::new_unck(comp_index);

                    let comp = node.components.get_mut(comp_index);
                    let script = comp.script;
                    let script = engine.script_manager.script(script);

                    if script.name == name {
                        let val = match !comp.is_ready {
                            true => {
                                info!("get_component: '{name}' wasn't ready");
                                comp.is_ready = true;
                                Some((script.functions.clone(), script.path()))
                            },


                            false => None,
                        };

                        break 'b (node.userdata_of(comp_index), val)
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
            let mut engine = Engine::generate();
            let mut engine = engine.get_mut();
            let node = engine.scene_manager.tree.get(this.0);

            let target = node.children[idx];
            let target = &engine.scene_manager.tree.get_mut(target).userdata();
            Ok(target.clone())
        });
    }

}


impl mlua::UserData for NodeId {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("scale", |_, this| Ok(Engine::generate().get().scene_manager.tree.get(*this).properties.scale));
    }


    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_component", |_, this, name: String| {
            let comp = 'b: {
                let mut engine = Engine::generate();
                let mut engine = engine.get_mut();
                let engine = &mut *engine;
                let node = engine.scene_manager.tree.get_mut(*this);

                let mut comp_index = 0u32;
                loop {
                    comp_index += 1;
                    let comp_index = comp_index - 1;
                    
                    if comp_index as usize >= node.components.len() {
                        break
                    }

                    let comp_index = ComponentId::new_unck(comp_index);

                    let comp = node.components.get_mut(comp_index);
                    let script = comp.script;
                    let script = engine.script_manager.script(script);

                    if script.name == name {
                        let val = match !comp.is_ready {
                            true => {
                                info!("get_component: '{name}' wasn't ready");
                                comp.is_ready = true;
                                Some((script.functions.clone(), script.path()))
                            },


                            false => None,
                        };

                        break 'b (node.userdata_of(comp_index), val)
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
    fn from_lua(og_value: Value, _: &mlua::Lua) -> mlua::Result<Self> {
        let value = og_value.as_userdata();
        let Some(value) = value
        else {
            return Err(Error::runtime(format!("expected a 'NodeId' found '{:?}'", og_value.type_name())));
        };

        if let Ok(value) = value.borrow::<NodeUserData>() {
            return Ok(value.0)
        }

        value.borrow::<Self>().map(|x| *x)
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
