use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};

use genmap::Handle;
use mlua::{AnyUserData, Lua};
use rapier2d::prelude::{ActiveEvents, CCDSolver, Collider, ColliderBuilder, ColliderHandle, ColliderSet, CollisionEvent, DefaultBroadPhase, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryPipeline, RigidBody, RigidBodyBuilder, RigidBodyHandle, RigidBodySet};
use tracing::{error, info};

use crate::{engine::{Engine, Timers}, lua::node::NodeUserData, math::vector::Vec2, scene_manager::{scene_tree::SceneTree, NodeId}};

pub struct PhysicsServer {
    pub gravity: Vec2,
    integration_params: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,

    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    collider_userdata: HashMap<ColliderId, ColliderData>,
    rigidbody_userdata: HashMap<RigidBodyId, AnyUserData>,
    pub node_to_rigidbody: HashMap<NodeId, RigidBodyId>,

    last_tick: u64,
}


impl PhysicsServer {
    pub fn new(gravity: Vec2) -> Self {
        Self {
            gravity,
            integration_params: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(), 
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),

            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            collider_userdata: HashMap::new(),
            rigidbody_userdata: HashMap::new(),
            node_to_rigidbody: HashMap::new(),
            last_tick: 0,
        }
    }


    pub fn set_framerate(&mut self, fps: usize) {
        info!("set physics framerate to {fps}");

        let dt = 1.0/fps as f32;
        self.integration_params.dt = dt;
    }


    pub fn init(&mut self) {
        self.last_tick = sokol::time::now();
    }


    pub fn collider_cuboid(&mut self, lua: &Lua, node: NodeUserData, scale: Vec2) -> (ColliderId, AnyUserData) {
        info!("creating a cuboid collider '{scale}'");
        let id = ColliderId(self.collider_set.insert(ColliderBuilder::cuboid(scale.x, scale.y).active_events(ActiveEvents::COLLISION_EVENTS)));
        let userdata = lua.create_userdata(id).unwrap();
        let collider_data = ColliderData { events: vec![], node };
        self.collider_userdata.insert(id, collider_data);
        (id, userdata)
    }


    pub fn create_kinematic_rigidbody(&mut self, lua: &Lua, owner: NodeId) -> (RigidBodyId, AnyUserData) {
        info!("creating a kinematic rigid body");
        let userdata = unsafe { core::mem::transmute([owner.0.gen, owner.0.idx]) };
        let id = RigidBodyId(self.rigid_body_set.insert(RigidBodyBuilder::kinematic_velocity_based().user_data(userdata).build()));
        let userdata = lua.create_userdata(id).unwrap();
        self.rigidbody_userdata.insert(id, userdata.clone());
        self.node_to_rigidbody.insert(owner, id);
        (id, userdata)
    }


    pub fn create_dynamic_rigidbody(&mut self, lua: &Lua, owner: NodeId) -> (RigidBodyId, AnyUserData) {
        info!("creating a dynamic rigid body");
        let userdata = unsafe { core::mem::transmute([owner.0.gen, owner.0.idx]) };
        let id = RigidBodyId(self.rigid_body_set.insert(RigidBodyBuilder::dynamic().user_data(userdata).build()));
        let userdata = lua.create_userdata(id).unwrap();
        self.rigidbody_userdata.insert(id, userdata.clone());
        self.node_to_rigidbody.insert(owner, id);
        (id, userdata)
    }


    pub fn create_static_rigidbody(&mut self, lua: &Lua) -> (RigidBodyId, AnyUserData) {
        info!("creating a static rigid body");
        let id = RigidBodyId(self.rigid_body_set.insert(RigidBodyBuilder::new(rapier2d::prelude::RigidBodyType::Fixed).build()));
        let userdata = lua.create_userdata(id).unwrap();
        self.rigidbody_userdata.insert(id, userdata.clone());
        (id, userdata)
    }


    pub fn attach_collider_to_rigidbody(&mut self, cl: ColliderId, rb: RigidBodyId) {
        info!("attaching {rb:?} to {cl:?}");
        self.collider_set.set_parent(cl.0, Some(rb.0), &mut self.rigid_body_set);
    }


    pub fn delete_collider(&mut self, collider: ColliderId) {
        self.collider_userdata.remove(&collider);
        self.collider_set.remove(collider.0, &mut self.island_manager, &mut self.rigid_body_set, true);
    }

    
    pub fn delete_rb(&mut self, rbid: RigidBodyId) {
        let rb = self.rigid_body_set.get(rbid.0).unwrap();
        if !rb.is_fixed() {
            let [gen, idx] = unsafe { core::mem::transmute(rb.user_data) };
            let node = NodeId(Handle { gen, idx });
            self.node_to_rigidbody.remove(&node);
        }

        self.rigidbody_userdata.remove(&rbid);
        self.rigid_body_set.remove(rbid.0, &mut self.island_manager, &mut self.collider_set, &mut self.impulse_joint_set, &mut self.multibody_joint_set, false);
    }


    pub fn attach_collider_event(&mut self, cl: ColliderId, func: mlua::Function) {
        self.collider_userdata.get_mut(&cl).unwrap().events.push(func);
    }


    pub fn get_collider(&self, id: ColliderId) -> &Collider {
        self.collider_set.get(id.0).unwrap()
    }


    pub fn get_rb(&self, id: RigidBodyId) -> &RigidBody {
        self.rigid_body_set.get(id.0).unwrap()
    }


    pub fn get_rb_mut(&mut self, id: RigidBodyId) -> &mut RigidBody {
        self.rigid_body_set.get_mut(id.0).unwrap()
    }


    //pub fn tick(&mut self, scene: &mut SceneTree, timers: &mut Timers) -> Vec<(mlua::Function, NodeUserData, NodeUserData)> {
    pub fn tick(engine: &mut Engine) -> Vec<(mlua::Function, NodeUserData, NodeUserData)> {
        let timer = Instant::now();

        let mut engine_handle = engine.get_mut();
        let engine_ref = &mut *engine_handle;
        let physics = &engine_ref.scene_manager.physics;
        let physics_dt = physics.integration_params.dt as f64;
        let physics_dt = physics_dt * 1000000000.0;
        let physics_dt = physics_dt as u64;

        let now = sokol::time::now();
        let mut time_since_last_tick = sokol::time::diff(now, physics.last_tick);

        let event_handler = EventHandler {
            calls: Mutex::new(vec![]),
        };

        {
            let dt = engine_ref.dt;
            engine_ref.dt = physics.integration_params.dt;
            engine_ref.timers.physics_engine_conv_time = Duration::from_secs(0);
            drop(engine_handle);
            let timer = Instant::now();
            let mut num = 0;

            while time_since_last_tick >= physics_dt {
                let mut engine_handle = engine.get_mut();
                let nodes = engine_handle.scene_manager.tree.iter_vec_root();

                let physics = &mut engine_handle.scene_manager.physics;

                physics.physics_pipeline.step(
                    &physics.gravity.into(),
                    &physics.integration_params,
                    &mut physics.island_manager,
                    &mut physics.broad_phase,
                    &mut physics.narrow_phase,
                    &mut physics.rigid_body_set,
                    &mut physics.collider_set,
                    &mut physics.impulse_joint_set,
                    &mut physics.multibody_joint_set,
                    &mut physics.ccd_solver,
                    Some(&mut physics.query_pipeline),
                    &(),
                    &event_handler,
                );

     
                {
                    let timer = Instant::now();
                    let mut to_be_removed = vec![];
                    let engine_ref = &mut *engine_handle;

                    let tree = &mut engine_ref.scene_manager.tree;
                    let physics = &mut engine_ref.scene_manager.physics;

                    for (node_id, rb) in physics.node_to_rigidbody.iter() {
                        if !tree.exists(*node_id) {
                            error!("the node '{node_id:?}' is attached to '{rb:?}' \
                                   but the node was freed without detaching from the \
                                   rigidbody. detaching.");
                            to_be_removed.push(*node_id);
                        }

                        let rb = physics.rigid_body_set.get(rb.0).unwrap();
                        if rb.is_fixed() { continue }

                        let pos = rb.position();
                        let pos = Vec2::new(pos.translation.x, pos.translation.y);
                        let rot = rb.rotation().angle();

                        tree.set_global_position(*node_id, pos);
                        tree.set_global_rotation(*node_id, rot);
                    }


                    for node in to_be_removed.iter() {
                        engine_ref.scene_manager.physics.node_to_rigidbody.remove(&node);
                    }

                    engine_ref.timers.physics_engine_conv_time += timer.elapsed();
                }

                drop(engine_handle);

                for node in nodes.iter().copied() {
                    let comps = {
                        let mut engine = engine.get_mut();
                        let node = engine.scene_manager.tree.get_mut(node);
                        node.components.iter()
                    };
                    
                    for comp in comps {
                        let (functions, userdata, path) = {
                            let mut engine = engine.get_mut();
                            let node = engine.scene_manager.tree.get_mut(node);
                            let userdata = node.userdata_of(comp).clone();

                            let component = node.components.get(comp);
                            let script = component.script;
                            let script = engine.script_manager.script(script);

                            (
                                script.functions.clone(),
                                userdata,
                                script.path(),
                            )
                        };


                        functions.physics_update(path, userdata);
                    }


                }


                time_since_last_tick -= physics_dt;
                num += 1;
            }

            let mut engine_handle = engine.get_mut();
            let engine_ref = &mut *engine_handle;
            let physics = &mut engine_ref.scene_manager.physics;

            if num > 0 {
                physics.last_tick = now;
                physics.last_tick -= time_since_last_tick;
            }

            engine_ref.dt = dt;
            engine_ref.timers.physics_engine_iter_amount = num;
            engine_ref.timers.physics_engine_physics_time = timer.elapsed();
        }

        let mut engine_handle = engine.get_mut();
        let engine_ref = &mut *engine_handle;

        let mut vec = vec![];
        {
            let physics = &mut engine_ref.scene_manager.physics;
            let timer = Instant::now();

            // @PERFORMANCE: might wanna cache this vec
            // note: bro is that really the only problem here
            for (c1, c2) in event_handler.calls.lock().unwrap().iter() {
                let c1d = physics.collider_userdata.get(c1).unwrap();
                let c2d = physics.collider_userdata.get(c2).unwrap();

                for e in c1d.events.iter() {
                    vec.push((e.clone(), c1d.node.clone(), c2d.node.clone()));
                }

                for e in c2d.events.iter() {
                    vec.push((e.clone(), c2d.node.clone(), c1d.node.clone()));
                }
            }

            engine_ref.timers.physics_engine_event_time = timer.elapsed();
        }

        engine_ref.timers.physics_engine_time = timer.elapsed();
        vec
    }

}


#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct ColliderId(ColliderHandle);

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct RigidBodyId(RigidBodyHandle);


#[derive(Debug)]
pub struct ColliderData {
    node: NodeUserData,
    events: Vec<mlua::Function>,
}


impl core::fmt::Debug for PhysicsServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PhysicsServer")
    }
}

impl core::fmt::Debug for ColliderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "collider{:?}", self.0.0.into_raw_parts())
    }
}
impl core::fmt::Debug for RigidBodyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rigidbody{:?}", self.0.0.into_raw_parts())
    }
}


struct EventHandler {
    calls: Mutex<Vec<(ColliderId, ColliderId)>>,
}


impl rapier2d::prelude::EventHandler for EventHandler {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: rapier2d::prelude::CollisionEvent,
        _contact_pair: Option<&rapier2d::prelude::ContactPair>,
    ) {
        let mut lock = self.calls.lock().unwrap();
        match event {
            CollisionEvent::Started(collider_handle, collider_handle1, ..) => {
                lock.push((ColliderId(collider_handle), ColliderId(collider_handle1)));
            },
            CollisionEvent::Stopped(..) => (),
        }

    }

    fn handle_contact_force_event(
        &self,
        _dt: f32,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _contact_pair: &rapier2d::prelude::ContactPair,
        _total_force_magnitude: f32,
    ) {
    }
}


impl rapier2d::prelude::PhysicsHooks for EventHandler {
    fn filter_contact_pair(&self, _context: &rapier2d::prelude::PairFilterContext) -> Option<rapier2d::prelude::SolverFlags> {
        Some(rapier2d::prelude::SolverFlags::COMPUTE_IMPULSES)
    }

    fn filter_intersection_pair(&self, _context: &rapier2d::prelude::PairFilterContext) -> bool {
        true
    }

    fn modify_solver_contacts(&self, _context: &mut rapier2d::prelude::ContactModificationContext) {
    }
}


impl From<Vec2> for rapier2d::math::Vector<f32> {
    fn from(value: Vec2) -> Self {
        Self::new(value.x, value.y)
    }
}


impl From<rapier2d::math::Vector<f32>> for Vec2 {
    fn from(value: rapier2d::math::Vector<f32>) -> Self {
        Self::new(value.x, value.y)
    }
}
