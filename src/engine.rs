use std::{cell::{Cell, RefCell}, ptr::null, time::{Duration, Instant}};

use sokol::{debugtext as sdtx, app as sapp, time as stime};
use tracing::{error, info, trace, Level};

use crate::{asset_manager::AssetManager, event_manager::{EventManager, Keycode}, input_manager::InputManager, lua, math::vector::{Vec2, Vec3, Vec4}, renderer::Renderer, scene_manager::{node::NodeProperties, scene_tree::SceneTree, SceneManager}, script_manager::ScriptManager, settings::ProjectSettings, timer::Timer, Camera};


static mut ENGINE : *const Engine = null();


pub struct EngineStatic {
    engine: RefCell<Engine>,
    lua: mlua::Lua,
}


#[derive(Debug)]
pub struct Engine {
    pub project_settings: ProjectSettings,
    pub event_manager: RefCell<EventManager>,
    pub input_manager: RefCell<InputManager>,
    pub script_manager: RefCell<ScriptManager>,
    pub asset_manager: RefCell<AssetManager>,
    pub scene_manager: RefCell<SceneManager>,

    pub renderer: RefCell<Renderer>,

    pub last_frame: Cell<u64>,
    pub now: Cell<f32>,
    pub dt: Cell<f32>,
    pub show_colliders: Cell<bool>,
    pub timers: Timers,

    pub camera: RefCell<Camera>,
    pub lua: mlua::Lua,
}


#[derive(Debug, Default)]
pub struct Timers {
    pub node_update_time: Cell<Duration>,
    pub node_event_time: Cell<Duration>,
    pub node_render_time: Cell<Duration>,

    pub physics_engine_time: Cell<Duration>,
    pub physics_engine_physics_time: Cell<Duration>,
    pub physics_engine_conv_time: Cell<Duration>,
    pub physics_engine_event_time: Cell<Duration>,

    pub io_event_time: Cell<Duration>,

    pub frame_update_time: Cell<Duration>,
    pub frame_render_time: Cell<Duration>,
}


impl Engine {
    pub fn new(project_settings: ProjectSettings) -> &'static Engine {
        info!("creating engine");
        if !unsafe { ENGINE.is_null() } { 
            error!("there already is an engine instance");
            return Engine::get()
        }

        let slf = Self {
            project_settings: project_settings.clone(),
            event_manager: RefCell::new(EventManager::new()),
            script_manager: RefCell::new(ScriptManager::new()),
            input_manager: RefCell::new(InputManager::new()),
            asset_manager: RefCell::new(AssetManager::new()),
            scene_manager: RefCell::new(SceneManager::new(project_settings.world.gravity)),
            renderer: RefCell::new(Renderer::new()),
            camera: RefCell::new(Camera::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 25.0)),

            lua: mlua::Lua::new(),
            last_frame: Cell::new(0),
            now: Cell::new(0.0),
            dt: Cell::new(0.0),
            show_colliders: Cell::new(false),
            timers: Timers::default().into(),
        };


        let b = Box::leak(Box::new(slf));
        unsafe { ENGINE = b };

        Engine::get()

    }


    pub fn get() -> &'static Engine {
        assert!(unsafe { !ENGINE.is_null() });
        unsafe { &*ENGINE }
    }


    pub fn change_scene(&self, scene: &str) {
        self.scene_manager.borrow_mut().load(scene);

        info!("ready all nodes");
        let nodes = self.scene_manager.borrow().current.iter_vec();

        for handle in nodes {
            let comps = self.scene_manager.borrow().current.get(handle).borrow().components.clone();
            for (comp_id, _) in comps.iter() {
                let (was_ready, script, user_data) = {
                    let sm = self.scene_manager.borrow_mut();
                    let mut node = sm.current.get(handle).borrow_mut();
                    let comp = node.get_comp_mut(comp_id);
                    let was_ready = comp.is_ready;
                    comp.is_ready = true;
                    (was_ready, comp.script, comp.userdata.clone())
                };

                if was_ready { continue }

                let sm = self.script_manager.borrow(); 
                let script = sm.script(script);
                script.functions.ready(script.path(), &user_data);
            }
        }
    }


    // this is called after all the sokol stuff is initialised
    pub fn init(&self) {
        info!("intializing engine");

        {
            info!("set up lua environment");
            lua::setup_lua_environment(&self.lua);
        }

        self.asset_manager.borrow_mut().init();
        self.script_manager.borrow_mut().load_current_dir();
        
        let scene_tree = SceneTree::from_file(&self.project_settings.world.entry_scene);
        self.scene_manager.borrow_mut().register(self.project_settings.world.entry_scene.clone(), scene_tree);

        self.last_frame.set(stime::now());

        self.change_scene(&self.project_settings.world.entry_scene);
    }


    pub fn update(&self) {
        let _timer = Timer::new(&self.timers.frame_update_time);
        // update timers
        {
            let now = stime::now();
            let dt = stime::diff(now, self.last_frame.get());
            self.last_frame.set(now);

            let now = stime::sec(now) as f32;
            let dt = stime::sec(dt) as f32;

            self.dt.set(dt);
            self.now.set(now);
        }

        {
            let _timer = Timer::new(&self.timers.io_event_time);

            let mut event_manager = self.event_manager.borrow_mut();
            self.input_manager.borrow_mut().process(event_manager.event_queue());

            event_manager.clear_queue();
            drop(event_manager);
        }

        let mut sm = self.scene_manager.borrow_mut();
        let smr = &mut *sm;
        let events = smr.physics.tick(self.dt.get(), &smr.current, &self.timers);

        {
            trace!("update all nodes");

            let _timer = Timer::new(&self.timers.node_update_time);
            let nodes = smr.current.iter_vec();
            drop(sm);

            for handle in nodes {
                let comps = self.scene_manager.borrow().current.get(handle).borrow()
                            .components.clone();

                for (comp_id, _) in comps.iter() {
                    let (script, user_data) = {
                        let sm = self.scene_manager.borrow_mut();
                        let mut node = sm.current.get(handle).borrow_mut();
                        let comp = node.get_comp_mut(comp_id);
                        (comp.script, comp.userdata.clone())
                    };

                    let sm = self.script_manager.borrow(); 
                    let script = sm.script(script);
                    let funcs = script.functions.clone();
                    let path = script.path();
                    drop(sm);

                    funcs.update(path, user_data);
                }
            }
        }
            
        {
            trace!("call events");

            let _timer = Timer::new(&self.timers.node_event_time);
            for event in events.into_iter() {
                event.0.call::<()>((event.1, event.2)).unwrap();
            }
        }


        let im = self.input_manager.borrow();
        if im.is_key_down(Keycode::LeftControl)
            && im.is_key_down(Keycode::LeftShift)
            && im.is_key_just_pressed(Keycode::Z) {
            self.show_colliders.set(!self.show_colliders.get());
            info!("show debug colliders: {}", self.show_colliders.get());
        }
    }


    pub fn render(&self) {
        let span = tracing::span!(Level::TRACE, "render");
        let _handle = span.entered();

        let timer = Timer::new(&self.timers.frame_render_time);

        let (width, height) = (sapp::widthf(), sapp::heightf());
        let aspect_ratio = width/height;

        // render
        let mut renderer = self.renderer.borrow_mut();
        
        renderer.set_camera(&self.camera.borrow(), aspect_ratio);

        renderer.begin_frame();
        // render nodes
        {
            let span = tracing::span!(Level::TRACE, "nodes");
            let _handle = span.entered();
            trace!("started rendering nodes");

            let _timer = Timer::new(&self.timers.node_render_time);

            let mut stack = vec![];
            let mut property_stack = vec![(1, NodeProperties::identity())];
            let scene = self.scene_manager.borrow();

            if let Some(root) = scene.current.root() { stack.push(root); }

            while let Some(id) = stack.pop() {
                let span = tracing::span!(Level::TRACE, "", node = id.idx());
                let _handle = span.entered();

                let node = scene.current.get(id).borrow();
                let parent_properties = {
                    let props = property_stack.last_mut().unwrap();
                    props.0 -= 1;
                    if props.0 == 0 { property_stack.pop().unwrap().1 }
                    else { props.1 }
                };

                let properties = node.properties.merge(parent_properties);

                // add children to the render queue
                if node.children.len() != 0 {
                    trace!("adding {} children to the render queue", node.children.len());
                    stack.extend_from_slice(&node.children);
                    property_stack.push((node.children.len(), properties));
                }

                let model = renderer.draw_quad()
                    .position(properties.position)
                    .scale(properties.scale)
                    .rotation(properties.rotation)
                    .modulate(properties.modulate);

                let mvp = if let Some(texture) = properties.texture {
                    let model = model.texture(texture);
                    model.commit()
                } else {
                    model.mvp()
                };

                // call the 'draw' functions of the components
                let _draw = lua::draw::Draw::register(mvp, &mut renderer);
                let comps = node.components.clone();

                drop(node);
                for (_, comp) in comps.iter() {
                    let sm = self.script_manager.borrow(); 
                    let script = sm.script(comp.script);
                    script.functions.draw(script.path(), comp.userdata.clone());
                }
            }
        }


        // draw colliders

        if self.show_colliders.get() {
            trace!("draw colliders");

            for (_, coll) in self.scene_manager.borrow().physics.collider_set.iter() {
                let pos = Vec2::new(coll.position().translation.x, coll.position().translation.y);
                let shape = coll.shape().as_cuboid().unwrap().half_extents;
                let angle = coll.rotation().angle();
                let scale = Vec2::new(shape.x, shape.y);

                renderer.draw_quad()
                    .position(pos)
                    .rotation(angle)
                    .scale(scale)
                    .modulate(Vec4::new(0.0, 0.4, 0.4, 0.4))
                    .commit();
            }
        }

        drop(timer);

        // debug text
        trace!("draw debug text");
        sdtx::canvas(sapp::widthf() * 0.5, sapp::heightf() * 0.5);
        sdtx::font(0);
        sdtx::color3f(0.0, 0.0, 0.0);
        sdtx::puts(&format!("{} FPS", (1.0/self.dt.get()) as u64));
        sdtx::crlf();
        sdtx::puts(&format!("CAMERA: {}", self.camera.borrow().position));
        sdtx::crlf();
        sdtx::puts(&format!("WINDOW: {}x{}", sapp::widthf(), sapp::heightf()));
        sdtx::crlf();
        sdtx::puts(&format!("ASPECT RATIO: {}", aspect_ratio));
        sdtx::crlf();
        sdtx::puts(&format!("ORTHO: {}", self.camera.borrow().ortho));
        sdtx::crlf();
        sdtx::puts(&format!("DRAW COUNT: {}", renderer.draw_calls));
        sdtx::crlf();
        sdtx::crlf();
        let timers = &self.timers;
        sdtx::puts(&format!("TIMERS"));
        sdtx::crlf();
        sdtx::puts(&format!("FRAME TIME: {}", timers.frame_update_time.get().as_micros() 
                                                + timers.frame_render_time.get().as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- UPDATE TIME: {}", timers.frame_update_time.get().as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- RENDER TIME: {}", timers.frame_render_time.get().as_micros()));
        sdtx::crlf();

        sdtx::puts(&format!("NODE TIME: {}", timers.node_update_time.get().as_micros()
                                                + timers.node_event_time.get().as_micros()
                                                + timers.node_render_time.get().as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- UPDATE TIME: {}", timers.node_update_time.get().as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- EVENT TIME: {}", timers.node_event_time.get().as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- RENDER TIME: {}", timers.node_render_time.get().as_micros()));
        sdtx::crlf();

        sdtx::puts(&format!("PHYSICS TIME: {}", timers.physics_engine_time.get().as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- STEP TIME: {}", timers.physics_engine_physics_time.get().as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- CONVERTION TIME: {}", timers.physics_engine_conv_time.get().as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- EVENT TIME: {}", timers.physics_engine_event_time.get().as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("IO EVENT TIME: {}", timers.io_event_time.get().as_micros()));
        sdtx::crlf();
        sdtx::draw();

        renderer.end_frame();
    }
}


