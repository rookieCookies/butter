use std::{cell::{Ref, RefCell, RefMut}, ptr::null, time::{Duration, Instant}};

use mlua::{Compiler, Function};
use sokol::{debugtext as sdtx, app as sapp, time as stime};
use tracing::{error, info, trace, Level};

use crate::{asset_manager::AssetManager, event_manager::{EventManager, Keycode}, input_manager::InputManager, lua::{self}, math::vector::{Colour, Vec2, Vec3, Vec4}, physics::PhysicsServer, renderer::Renderer, scene_manager::{node::NodeProperties, scene_template::TemplateScene, scene_tree::SceneTree, SceneManager}, script_manager::ScriptManager, settings::ProjectSettings, Camera};


static mut ENGINE : *const EngineStatic = null();


pub struct EngineStatic {
    engine: RefCell<ManagerManager>,
    project_settings: ProjectSettings,
    lua: mlua::Lua,
}


pub struct Engine {}


#[derive(Debug)]
pub struct ManagerManager {
    pub event_manager: EventManager,
    pub input_manager: InputManager,
    pub script_manager: ScriptManager,
    pub asset_manager: AssetManager,
    pub scene_manager: SceneManager,

    pub renderer: Renderer,

    pub last_frame: u64,
    pub now: f32,
    pub dt: f32,
    pub show_colliders: bool,
    pub timers: Timers,

    pub camera: Camera,
}


#[derive(Debug, Default)]
pub struct Timers {
    pub node_update_time: Duration,
    pub node_event_time: Duration,
    pub node_render_time: Duration,

    pub physics_engine_time: Duration,
    pub physics_engine_physics_time: Duration,
    pub physics_engine_conv_time: Duration,
    pub physics_engine_event_time: Duration,
    pub physics_engine_iter_amount: usize,

    pub io_event_time: Duration,

    pub frame_update_time: Duration,
    pub frame_render_time: Duration,
}


impl Engine {
    pub fn new(project_settings: ProjectSettings) {
        info!("creating engine");
        if !unsafe { ENGINE.is_null() } { 
            error!("there already is an engine instance");
            return
        }

        let slf = ManagerManager {
            event_manager: EventManager::new(),
            script_manager: ScriptManager::new(),
            input_manager: InputManager::new(),
            asset_manager: AssetManager::new(),
            scene_manager: SceneManager::new(project_settings.world.gravity),
            renderer: Renderer::new(&project_settings),
            camera: Camera::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 25.0),

            last_frame: 0,
            now: 0.0,
            dt: 0.0,
            show_colliders: false,
            timers: Timers::default(),
        };

        let slf = EngineStatic {
            engine: slf.into(),
            lua: mlua::Lua::new(),
            project_settings,
        };

        slf.lua.globals().set("not_set", 0).unwrap();

        slf.lua.set_compiler(Compiler::new()
                             .set_debug_level(2)
                             .set_coverage_level(2));

        let b = Box::leak(Box::new(slf));
        unsafe { ENGINE = b };

    }


    pub fn project_settings() -> &'static ProjectSettings {
        assert!(unsafe { !ENGINE.is_null() });
        unsafe { &(*ENGINE).project_settings }

    }


    pub fn lua() -> &'static mlua::Lua {
        assert!(unsafe { !ENGINE.is_null() });
        unsafe { &(*ENGINE).lua }
    }


    pub fn change_scene(engine: &mut Engine, scene: &str) {
        let template_id = SceneManager::template_from_file(engine, scene);

        let Some(node) = TemplateScene::instantiate(engine, template_id)
        else { return };

        SceneTree::set_root(engine, node);
    }


    // this is called after all the sokol stuff is initialised
    pub fn init(engine: &mut Engine) {
        info!("intializing engine");

        {
            info!("set up lua environment");
            lua::setup_lua_environment(Engine::lua());
        }

        engine.with(|engine| {
            engine.asset_manager.init();
            engine.scene_manager.physics.init();
            let fps = Engine::project_settings()
                .world.physics_framerate;
            engine.scene_manager.physics.set_framerate(fps);
        });

        ScriptManager::load_current_dir(engine);

        SceneManager::init_templates(engine);

        Engine::change_scene(engine, &Engine::project_settings().world.entry_scene);
    }


    pub fn update(engine: &mut Engine) {
        let timer = Instant::now();

        // update timers
        engine.with(|engine| {
            let now = stime::now();
            let dt = stime::diff(now, engine.last_frame);
            engine.last_frame = now;

            let now = stime::sec(now) as f32;
            let dt = stime::sec(dt) as f32;

            engine.dt = dt;
            engine.now = now;
        });


        engine.with(|engine| {
            let timer = Instant::now();

            engine.input_manager.process(engine.event_manager.event_queue());

            engine.event_manager.clear_queue();

            engine.timers.io_event_time = timer.elapsed();
        });

        let nodes = engine.with(|engine| {
            engine.scene_manager.tree.iter_vec_root()
        });

        {
            trace!("update all nodes");

            let timer = Instant::now();
            
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


                    functions.update(path, userdata);
                }

            }

            trace!("updated");
            engine.with(|engine|
                         engine.timers.node_update_time = timer.elapsed());
        }
        

        let events = PhysicsServer::tick(engine);

        {
            trace!("call events");

            let timer = Instant::now();
            for event in events.into_iter() {
                event.0.call::<()>((event.1, event.2)).unwrap();
            }

            engine.with(|engine|
                         engine.timers.node_event_time = timer.elapsed());
        }


        {
            engine.with(|engine| {
                trace!("actually freeing nodes that were queue freed");
                for handle in nodes.iter().copied() {
                    let node = engine.scene_manager.tree.map.get(handle.0).unwrap();
                    if node.queued_free {
                        engine.scene_manager.tree.map.remove(handle.0).unwrap();
                    }
                }
                trace!("finished actually freeing nodes that were queue freed");
            });
        }


        engine.with(|engine| {
            let im = &engine.input_manager;
            if im.is_key_down(Keycode::LeftControl)
                && im.is_key_down(Keycode::LeftShift)
                && im.is_key_just_pressed(Keycode::Z) {
                engine.show_colliders = !engine.show_colliders;
                info!("show debug colliders: {}", engine.show_colliders);
            }
        });


        engine.with(|engine|
                     engine.timers.frame_update_time = timer.elapsed());
    }


    pub fn render(engine: &mut Engine) {
        let span = tracing::span!(Level::TRACE, "render");
        let _handle = span.entered();

        let timer = Instant::now();

        let (width, height) = (sapp::widthf(), sapp::heightf());
        let aspect_ratio = width/height;

        // begin render
        engine.with(|engine| {
            engine.renderer.set_camera(&engine.camera);
            engine.renderer.begin_frame();
            engine.renderer.clear_background(&engine.asset_manager, Colour::new(1.0, 1.0, 1.0, 1.0));
        });
        
        // render nodes
        {
            let span = tracing::span!(Level::TRACE, "nodes");
            let _handle = span.entered();
            trace!("started rendering nodes");

            let timer = Instant::now();

            let mut stack = vec![];
            let mut property_stack = vec![(1, NodeProperties::identity())];

            engine.with(|engine|
                if let Some(root) = engine.scene_manager.tree.root() {
                    stack.push(root)
                }
            );

            while let Some(node) = stack.pop() {
                let span = tracing::span!(Level::TRACE, "", node = node.idx());
                let _handle = span.entered();

                let mvp = {
                    let mut engine = engine.get_mut();
                    let engine = &mut *engine;

                    let node = engine.scene_manager.tree.get(node);
                    let parent_properties = {
                        let props = property_stack.last_mut().unwrap();
                        props.0 -= 1;
                        if props.0 == 0 { property_stack.pop().unwrap().1 }
                        else { props.1 }
                    };

                    let properties = node.properties.merge(parent_properties);

                    // add children to the render queue
                    if node.children.len() != 0 {
                        trace!("adding {} children to the render queue",
                               node.children.len());

                        stack.extend_from_slice(&node.children);
                        property_stack.push((node.children.len(), properties));
                    }

                    let model = engine.renderer.draw_quad()
                        .position(properties.position)
                        .scale(properties.scale)
                        .rotation(properties.rotation)
                        .modulate(properties.modulate);

                    let mvp = if let Some(texture) = properties.texture {
                        let model = model.texture(texture);
                        model.commit(&engine.asset_manager)
                    } else {
                        model.mvp()
                    };
                    
                    mvp
                };


                // call the 'draw' functions of the components
                trace!("draw functions");
                let camera_vp = engine.with(|engine| {
                    let old_vp = engine.renderer.vp;
                    engine.renderer.vp = mvp;
                    old_vp
                });

                lua::draw::Draw::register();


                let comps = {
                    let mut engine = engine.get_mut();
                    let node = engine.scene_manager.tree.get_mut(node);
                    node.components.iter()
                };

                for comp in comps {
                    let (functions, userdata, path) = {
                        let mut engine = engine.get_mut();
                        let node = engine.scene_manager.tree.get_mut(node);
                        let userdata = node.userdata_of(comp);

                        let component = node.components.get(comp);
                        let script = component.script;
                        let script = engine.script_manager.script(script);

                        (
                            script.functions.clone(),
                            userdata,
                            script.path(),
                        )
                    };


                    functions.draw(path, userdata);
                }
                trace!("draw functions done");

                lua::draw::Draw::unregister();
                engine.with(|engine| engine.renderer.vp = camera_vp);
            }


            engine.with(|engine|
                         engine.timers.node_render_time = timer.elapsed());
        }


        // draw colliders

        engine.with(|engine| {
            if engine.show_colliders {
                trace!("draw colliders");

                for (_, coll) in engine.scene_manager.physics.collider_set.iter() {
                    let pos = Vec2::new(coll.position().translation.x, coll.position().translation.y);
                    let shape = coll.shape().as_cuboid().unwrap().half_extents;
                    let angle = coll.rotation().angle();
                    let scale = Vec2::new(shape.x, shape.y);

                    engine.renderer
                        .draw_quad()
                        .position(pos)
                        .rotation(angle)
                        .scale(scale)
                        .modulate(Vec4::new(0.0, 0.4, 0.4, 0.4))
                        .commit(&engine.asset_manager);
                }
            }
        });

        engine.with(|engine|
                     engine.timers.frame_render_time = timer.elapsed());

        trace!("debug text");
        // debug text
        let mut engine = engine.get_mut();
        trace!("draw debug text");
        sdtx::font(0);
        sdtx::color3f(0.0, 0.0, 0.0);
        sdtx::puts(&format!("{} FPS", (1.0/engine.dt) as u64));
        sdtx::crlf();
        sdtx::puts(&format!("CAMERA: {}", engine.camera.position));
        sdtx::crlf();
        sdtx::puts(&format!("WINDOW: {}x{}", sapp::widthf(), sapp::heightf()));
        sdtx::crlf();
        sdtx::puts(&format!("ASPECT RATIO: {}", aspect_ratio));
        sdtx::crlf();
        sdtx::puts(&format!("ORTHO: {}", engine.camera.ortho));
        sdtx::crlf();
        sdtx::puts(&format!("DRAW COUNT: {}", engine.renderer.draw_calls));
        sdtx::crlf();
        sdtx::crlf();
        sdtx::puts(&format!("TIMERS"));
        sdtx::crlf();
        sdtx::puts(&format!("FRAME TIME: {}", engine.timers.frame_update_time.as_micros() 
                                                + engine.timers.frame_render_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- UPDATE TIME: {}", engine.timers.frame_update_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- RENDER TIME: {}", engine.timers.frame_render_time.as_micros()));
        sdtx::crlf();

        sdtx::puts(&format!("NODE TIME: {}", engine.timers.node_update_time.as_micros()
                                                + engine.timers.node_event_time.as_micros()
                                                + engine.timers.node_render_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- UPDATE TIME: {}", engine.timers.node_update_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- EVENT TIME: {}", engine.timers.node_event_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- RENDER TIME: {}", engine.timers.node_render_time.as_micros()));
        sdtx::crlf();

        sdtx::puts(&format!("PHYSICS TIME: {}", engine.timers.physics_engine_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- STEP TIME: {}", engine.timers.physics_engine_physics_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- CONVERTION TIME: {}", engine.timers.physics_engine_conv_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- EVENT TIME: {}", engine.timers.physics_engine_event_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("- ITER AMOUNT: {}", engine.timers.physics_engine_iter_amount));
        sdtx::crlf();
        sdtx::puts(&format!("IO EVENT TIME: {}", engine.timers.io_event_time.as_micros()));
        sdtx::crlf();
        sdtx::puts(&format!("INFO"));
        sdtx::crlf();
        sdtx::puts(&format!("RIGIDBODY COUNT: {}", engine.scene_manager.physics.rigid_body_set.len()));
        sdtx::crlf();
        sdtx::puts(&format!("COLLIDER COUNT: {}", engine.scene_manager.physics.collider_set.len()));
        sdtx::crlf();

        engine.renderer.end_frame();
    }
}


impl Engine {
    pub fn generate() -> Engine {
        Engine {}
    }


    pub fn get<'a>(&'a self) -> Ref<'a, ManagerManager> {
        assert!(unsafe { !ENGINE.is_null() });
        unsafe { (*ENGINE).engine.borrow() }
    }


    pub fn get_mut<'a>(&'a mut self) -> RefMut<'a, ManagerManager> {
        assert!(unsafe { !ENGINE.is_null() });
        unsafe { (*ENGINE).engine.borrow_mut() }
    }


    pub fn with<T, F: FnOnce(&mut ManagerManager) -> T>(&mut self, f: F) -> T {
        let mut engine = Self::get_mut(self);
        f(&mut engine)
    }
}

