#![feature(str_as_str)]
#![allow(unused_attributes)]

pub mod settings;
pub mod math;
pub mod input_manager;
pub mod event_manager;
pub mod script_manager;
pub mod asset_manager;
pub mod lua;
pub mod physics;
pub mod engine;
pub mod deserialize;
pub mod scene_manager;
pub mod renderer;
pub mod timer;

use core::str;
use std::{ffi::CString, process::exit};

use engine::Engine;
use math::vector::{Vec2, Vec3};
use sokol::{app as sapp, debugtext::{self as sdtx}, gfx::{self as sg, ImageSampleType, ImageType, SamplerType, ShaderStage, UniformLayout}, glue as sglue, time as stime};
use event_manager::{Event, Keycode, MouseButton};
use tracing::{error, info, warn};
use settings::{engine_version::EngineVersion, ProjectSettings};


const PROJECT_SETTINGS_FILE : &str = "project-settings.toml";


pub fn start() -> ! {
    let project_settings = {
        info!("reading project settings");
        let project_settings = match std::fs::read_to_string(PROJECT_SETTINGS_FILE) {
            Ok(v) => v,
            Err(_) => {
                error!("unable to read '{PROJECT_SETTINGS_FILE}'");
                exit(-2);
            },
        };

        let project_settings = match ProjectSettings::new(&project_settings) {
            Ok(v) => v,
            Err(_) => {
                error!("corrupt '{PROJECT_SETTINGS_FILE}'"); 
                exit(-2);
            },
        };

        project_settings
    };



    if EngineVersion::CURRENT != project_settings.engine.version {
        error!("this project was made on version '{}' but you are running version '{}'", project_settings.engine.version, EngineVersion::CURRENT);
        exit(-2);
    }


    Engine::new(project_settings.clone());
    info!("engine created");

    let title = to_cstring("window title", Engine::project_settings().window.title.clone());

    sapp::run(&sapp::Desc {
        init_cb: Some(init),
        frame_cb: Some(frame),
        event_cb: Some(event),

        window_title: title.as_ptr(),
        width: clamp_to_i32("window width", project_settings.window.width),
        height: clamp_to_i32("window height", project_settings.window.height),
        sample_count: clamp_to_i32("msaa sample count", project_settings.window.msaa_sample_count),

        high_dpi: project_settings.window.high_dpi,
        fullscreen: project_settings.window.fullscreen,
        alpha: project_settings.window.allow_transparency,
        ..Default::default()
    });
    unreachable!()
}



extern "C" fn init() {
    let mut engine = Engine::generate();

    sg::setup(&sg::Desc {
        environment: sglue::environment(),
        logger: sg::Logger {
            func: Some(sokol::log::slog_func),
            ..Default::default()
        },
        uniform_buffer_size: 4 * 1024 * 1024 * 10,
        ..Default::default()
    });

    stime::setup();

    {
        let mut desc = sdtx::Desc::default();
        desc.fonts[0] = sdtx::font_kc853();

        sdtx::setup(&desc);
        sdtx::canvas(Engine::project_settings().window.width as f32, Engine::project_settings().window.height as f32);
    }

    let mut engine_ref = engine.get_mut();
    let renderer = &mut engine_ref.renderer;
    // set up the quad for rendering
    {
        let verticies : [ModelVertex; 6] = [
            ModelVertex::new(Vec3::new(-1.0,   1.0,  0.0),   0.0,   0.0),
            ModelVertex::new(Vec3::new( 1.0,   1.0,  0.0),   1.0,   0.0),
            ModelVertex::new(Vec3::new( 1.0,  -1.0,  0.0),   1.0,   1.0),
            ModelVertex::new(Vec3::new(-1.0,   1.0,  0.0),   0.0,   0.0),
            ModelVertex::new(Vec3::new( 1.0,  -1.0,  0.0),   1.0,   1.0),
            ModelVertex::new(Vec3::new(-1.0,  -1.0,  0.0),   0.0,   1.0),
        ];


        renderer.bind.vertex_buffers[0] = sg::make_buffer(&sg::BufferDesc {
            data: sg::Range { ptr: verticies.as_ptr().cast(), size: verticies.len() * size_of::<ModelVertex>() },
            label: c"quad-verticies".as_ptr(),
            ..Default::default()
        });
    }


    // set up the texture
    {
        renderer.bind.samplers[0] = sg::make_sampler(&sg::SamplerDesc {
            wrap_u: sg::Wrap::ClampToEdge,
            wrap_v: sg::Wrap::ClampToEdge,
            wrap_w: sg::Wrap::ClampToEdge,
            ..Default::default()
        });
    }

    // set up the shader pipeline
    {
        let shd = sg::make_shader(&texcube_shader_desc(sg::query_backend()).unwrap());

        let mut pipeline = sg::PipelineDesc {
            shader: shd,
            ..Default::default()
        };

        pipeline.layout.attrs[0].format = sg::VertexFormat::Float3;
        pipeline.layout.attrs[1].format = sg::VertexFormat::Float2;
        pipeline.colors[0].write_mask = sg::ColorMask::Rgba;
        pipeline.colors[0].blend = sg::BlendState {
            enabled: true,
            src_factor_rgb: sg::BlendFactor::SrcAlpha,
            dst_factor_rgb: sg::BlendFactor::OneMinusSrcAlpha,
            ..Default::default()
        };
        pipeline.label = c"pipeline".as_ptr();
        renderer.render_pip = sg::make_pipeline(&pipeline);
    }

    // set background colour
    {
        renderer.pass_action.colors[0] = sg::ColorAttachmentAction {
            load_action: sg::LoadAction::Clear,
            clear_value: sg::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            ..Default::default()
        };
    }

    info!("using backend '{:?}'", sg::query_backend());

    drop(engine_ref);
    Engine::init(&mut engine);
}


extern "C" fn frame() {
    let mut engine = Engine::generate();

    Engine::update(&mut engine);
    Engine::render(&mut engine);
}


extern "C" fn event(event: *const sapp::Event) {
    let mut engine = Engine::generate();
    let event = unsafe { *event };

    let event = match event._type {
        sapp::EventType::Invalid => {
            error!("an invalid event was issued\
                   ignoring the event");
            return;
        },


        sapp::EventType::KeyDown => Event::KeyDown(Keycode::from_sokol(event.key_code), event.key_repeat),


        sapp::EventType::KeyUp => Event::KeyUp(Keycode::from_sokol(event.key_code), event.key_repeat),


        sapp::EventType::Char => {
            let ch = event.char_code.try_into();
            match ch {
                Ok(v) => Event::Character(v),
                Err(_) => {
                    error!("a character event was issued but the given\
                           char code ('{}') isn't a valid UTF-8 character.\
                           ignoring the event", event.char_code);
                    return;
                },
            }
        },


        sapp::EventType::MouseDown => Event::MouseDown(MouseButton::from_sokol(event.mouse_button)),
        sapp::EventType::MouseUp => Event::MouseUp(MouseButton::from_sokol(event.mouse_button)),
        sapp::EventType::MouseScroll => Event::MouseScroll(Vec2::new(event.scroll_x, event.scroll_y)),


        sapp::EventType::MouseMove => Event::MouseMove {
            abs: Vec2::new(event.mouse_x, event.mouse_y),
            delta: Vec2::new(event.mouse_dx, event.mouse_dy)
        },


        sapp::EventType::MouseEnter => Event::MouseEnter,
        sapp::EventType::MouseLeave => Event::MouseLeave,
        sapp::EventType::TouchesBegan => todo!(),
        sapp::EventType::TouchesMoved => todo!(),
        sapp::EventType::TouchesEnded => todo!(),
        sapp::EventType::TouchesCancelled => todo!(),
        sapp::EventType::Resized => Event::Resized,
        sapp::EventType::Iconified => Event::Minimised,
        sapp::EventType::Restored => Event::Restored,
        sapp::EventType::Focused => Event::Focused,
        sapp::EventType::Unfocused => Event::Unfocused,
        sapp::EventType::Suspended => Event::Suspended,
        sapp::EventType::Resumed => Event::Resumed,
        sapp::EventType::QuitRequested => Event::QuitRequested,
        sapp::EventType::ClipboardPasted => todo!(),
        sapp::EventType::FilesDropped => todo!(),
        sapp::EventType::Num => todo!(),
    };

    engine.get_mut().event_manager.push_event(event);
}


fn texcube_shader_desc(backend: sg::Backend) -> Option<sg::ShaderDesc> {
    if backend == sg::Backend::MetalMacos {
        let shader = concat!(include_str!("../shaders/shader.metal"), "\0");
        let mut desc = sg::ShaderDesc::new();
        desc.vertex_func.source = shader.as_ptr().cast();
        desc.vertex_func.entry = c"vs_main".as_ptr();
        desc.fragment_func.source = shader.as_ptr().cast();
        desc.fragment_func.entry = c"fs_main".as_ptr();
        desc.uniform_blocks[0].stage = ShaderStage::Vertex; // vertex sahder
        desc.uniform_blocks[0].layout = UniformLayout::Std140; // align type
        desc.uniform_blocks[0].size = 64; // f32x4x4
        desc.uniform_blocks[0].msl_buffer_n = 0; // no idea
        //fragment shader modulate uniform
        desc.uniform_blocks[1].stage = ShaderStage::Fragment; // fragment shader
        desc.uniform_blocks[1].layout = UniformLayout::Std140; // underlying type f32
        desc.uniform_blocks[1].size = 16; // vec4
        desc.uniform_blocks[1].msl_buffer_n = 0;
        // fragment shader texture uniform
        desc.images[0].stage = ShaderStage::Fragment; // fragment shader
        desc.images[0].image_type = ImageType::Dim2; // it's a 2d texture
        desc.images[0].sample_type = ImageSampleType::Float; // underlying type f32
        desc.images[0].multisampled = false; // idk
        desc.images[0].msl_texture_n = 0; // idk
        // fragment shader texture sampler
        desc.samplers[0].stage = ShaderStage::Fragment; // fragment shader
        desc.samplers[0].sampler_type = SamplerType::Filtering; // samoler
        desc.samplers[0].msl_sampler_n = 0; // idk
        desc.image_sampler_pairs[0].stage = ShaderStage::Fragment; // pair the two in the fragment shader??
        desc.image_sampler_pairs[0].image_slot = 0; // image at slot 0
        desc.image_sampler_pairs[0].sampler_slot = 0; // sampler at slot 0
        desc.label = c"texcube_shader".as_ptr();

        return Some(desc);
    }
    unimplemented!()
}


#[repr(C)]
struct ModelVertex {
    vertex: Vec3,
    u: f32,
    v: f32,
}


impl ModelVertex {
    fn new(vertex: Vec3, u: f32, v: f32) -> Self {
        Self { vertex, u, v }
    }
}


#[derive(Debug)]
pub struct Camera {
    position: Vec3,
    up: Vec3,
    ortho: f32,
}

impl Camera {
    fn new(position: Vec3, up: Vec3, ortho: f32) -> Self {
        Self { position, up, ortho }
    }
}


pub fn clamp_to_i32<T: TryInto<i32>>(name: &str, val: T) -> i32 {
    val.try_into()
       .unwrap_or_else(|_| { warn!("{name} is greater than i32::MAX, clamping to i32::MAX"); i32::MAX})
}


pub fn to_cstring(name: &str, mut val: String) -> CString {
    if val.contains('\0') {
        warn!("{name} contains nul bytes, replacing them with \\0");
        val = val.replace('\0', "\\0");
    }

    CString::new(val).unwrap()
}


