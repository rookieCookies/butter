use std::{collections::HashMap, sync::Mutex, time::Instant};

use mlua::{AnyUserData, Lua};
use rapier2d::{self as rape, prelude::{ActiveEvents, CCDSolver, Collider, ColliderBuilder, ColliderHandle, ColliderSet, CollisionEvent, DefaultBroadPhase, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryPipeline, RigidBody, RigidBodyBuilder, RigidBodyHandle, RigidBodySet}};
use tracing::info;

use crate::{engine::{Engine, Timers}, lua::node::NodeUserData, math::vector::Vec2, scene_manager::scene_tree::{NodeId, SceneTree}, timer::Timer};

pub struct PhysicsServer {
    pub gravity: rape::na::Vector2<f32>,
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
}


impl PhysicsServer {
    pub fn new(gravity: Vec2) -> Self {
        Self {
            gravity: rape::na::Vector2::new(gravity.x, gravity.y),
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
        }
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
        let id = RigidBodyId(self.rigid_body_set.insert(RigidBodyBuilder::kinematic_velocity_based().build()));
        let userdata = lua.create_userdata(id).unwrap();
        self.rigidbody_userdata.insert(id, userdata.clone());
        self.node_to_rigidbody.insert(owner, id);
        (id, userdata)
    }


    pub fn create_dynamic_rigidbody(&mut self, lua: &Lua, owner: NodeId) -> (RigidBodyId, AnyUserData) {
        info!("creating a dynamic rigid body");
        let id = RigidBodyId(self.rigid_body_set.insert(RigidBodyBuilder::dynamic().build()));
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
        self.collider_set.remove(collider.0, &mut self.island_manager, &mut self.rigid_body_set, true);
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


    pub fn tick(&mut self, time: f32, scene: &mut SceneTree, timers: &mut Timers) -> Vec<(mlua::Function, NodeUserData, NodeUserData)> {
        let timer = Instant::now();

        self.integration_params.dt = time;

        let event_handler = EventHandler {
            calls: Mutex::new(vec![]),
        };

        {
            let timer = Instant::now();

            self.physics_pipeline.step(
                &self.gravity,
                &self.integration_params,
                &mut self.island_manager,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.rigid_body_set,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                &mut self.ccd_solver,
                Some(&mut self.query_pipeline),
                &(),
                &event_handler,
            );

            timers.physics_engine_physics_time = timer.elapsed();
        }


        let mut vec = vec![];
        {
            let timer = Instant::now();

            // @PERFORMANCE: might wanna cache this vec
            // note: bro is that really the only problem here
            for (c1, c2) in event_handler.calls.lock().unwrap().iter() {
                let c1d = self.collider_userdata.get(c1).unwrap();
                let c2d = self.collider_userdata.get(c2).unwrap();

                for e in c1d.events.iter() {
                    vec.push((e.clone(), c1d.node.clone(), c2d.node.clone()));
                }
                for e in c2d.events.iter() {
                    vec.push((e.clone(), c2d.node.clone(), c1d.node.clone()));
                }
            }

            timers.physics_engine_event_time = timer.elapsed();
        }


        {
            let timer = Instant::now();

            for (node_id, rb) in self.node_to_rigidbody.iter() {
                let rb = self.rigid_body_set.get(rb.0).unwrap();
                //if rb.body_type().is_fixed() { continue }

                let pos = rb.position();
                let pos = Vec2::new(pos.translation.x, pos.translation.y);
                let rot = rb.rotation().angle();

                scene.set_global_position(*node_id, pos);
                scene.set_global_rotation(*node_id, rot);
            }

            timers.physics_engine_conv_time = timer.elapsed();
        }

        timers.physics_engine_time = timer.elapsed();
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
        dt: f32,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _contact_pair: &rapier2d::prelude::ContactPair,
        _total_force_magnitude: f32,
    ) {
        dbg!(dt);
    }
}


impl rapier2d::prelude::PhysicsHooks for EventHandler {
    fn filter_contact_pair(&self, _context: &rapier2d::prelude::PairFilterContext) -> Option<rapier2d::prelude::SolverFlags> {
        dbg!("hi");
        Some(rapier2d::prelude::SolverFlags::COMPUTE_IMPULSES)
    }

    fn filter_intersection_pair(&self, _context: &rapier2d::prelude::PairFilterContext) -> bool {
        dbg!("hi");
        true
    }

    fn modify_solver_contacts(&self, _context: &mut rapier2d::prelude::ContactModificationContext) {
        dbg!("hi");
    }
}
