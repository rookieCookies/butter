use sokol::gfx::{self as sg, Bindings, PassAction, Pipeline};
use tracing::{trace, Level};

use crate::{asset_manager::{AssetManager, TextureId}, engine::Engine, math::{matrix::{Matrix, Matrix4}, vector::{Vec2, Vec3, Vec4}}, Camera};

#[derive(Debug)]
pub struct Renderer {
    pub pass_action: PassAction,
    pub bind: Bindings,
    pub pip: Pipeline,

    pub vp : Matrix4<f32>,

    // stats
    pub draw_calls: usize,
}



impl Renderer {
    pub fn new() -> Self {
        Self {
            pass_action: PassAction::new(),
            bind: Bindings::new(),
            pip: Pipeline::new(),
            vp: Matrix4::IDENTITY,
            draw_calls: 0,
        }

    }


    pub fn set_camera(&mut self, camera: &Camera, aspect_ratio: f32) {
        let span = tracing::span!(Level::TRACE, "Renderer::set_camera");
        let _handle = span.entered();

        let view_proj = {
            trace!("create view projection matrix");
            let n = camera.ortho;
            let left = -n*0.5*aspect_ratio;
            let right = n*0.5*aspect_ratio;
            let down = -n*0.5;
            let up = n*0.5;

            let proj = Matrix::orthographic(
                left, right,
                down, up,
                -1.0, 1.0);

            let view = Matrix::look_at(
                            camera.position,
                            camera.position + Vec3::new(0.0, 0.0, -1.0),
                            camera.up);
            proj * view
        };

        trace!("updating the view projection matrix");
        self.vp = view_proj;
    }


    pub fn begin_frame(&mut self) {
        let span = tracing::span!(Level::TRACE, "Renderer::begin_frame");
        let _handle = span.entered();

        self.draw_calls = 0;

        trace!("begin pass");
        sg::begin_pass(&sg::Pass {
            action: self.pass_action,
            swapchain: sokol::glue::swapchain(),
            ..Default::default()
        });


        trace!("apply pipeline");
        sg::apply_pipeline(self.pip);
    }


    pub fn end_frame(&mut self) {
        trace!("end pass & commit");
        sg::end_pass();
        sg::commit();
    }


    pub fn draw_quad<'me>(&'me mut self) -> FrameQuad<'me> {
        FrameQuad::new(self)
    }
}


pub struct FrameQuad<'me> {
    renderer: &'me mut Renderer,
    pos: Vec2,
    scale: Vec2,
    rot: f32,
    texture: TextureId,
    modulate: Vec4,
}


impl<'me> FrameQuad<'me> {
    fn new(frame: &'me mut Renderer) -> Self {
        Self {
            renderer: frame,
            pos: Vec2::new(0.0, 0.0),
            scale: Vec2::new(1.0, 1.0),
            rot: 0.0,
            texture: TextureId::WHITE,
            modulate: Vec4::new(1.0, 1.0, 1.0, 1.0)
        }
    }


    pub fn position(mut self, pos: Vec2) -> Self {
        self.pos = pos;
        self
    }


    pub fn scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }


    pub fn rotation(mut self, rot: f32) -> Self {
        self.rot = rot;
        self
    }


    pub fn modulate(mut self, modulate: Vec4) -> Self {
        self.modulate = modulate;
        self
    }


    pub fn texture(mut self, texture: TextureId) -> Self {
        self.texture = texture;
        self
    }


    pub fn mvp(&self) -> Matrix4<f32> {
        let model = Matrix::pos_scale_rot(self.pos, self.scale, self.rot);
        self.renderer.vp * model
    }


    pub fn commit(self, asset_manager: &AssetManager) -> Matrix4<f32> {
        trace!("drawing a quad");
        trace!(" - position: {}", self.pos);
        trace!(" - scale   : {}", self.scale);
        trace!(" - rotation: {}", self.rot);
        trace!(" - modulate: {}", self.modulate);
        trace!(" - texture : {}", self.texture.inner());

        let model = Matrix::pos_scale_rot(self.pos, self.scale, self.rot);
        let mvp = self.renderer.vp * model;

        self.renderer.bind.images[0] = asset_manager.texture(self.texture).inner();
        sg::apply_bindings(&self.renderer.bind);

        sg::apply_uniforms(0, &sg::Range { ptr: ((&mvp) as *const Matrix4<f32>).cast(), size: 64 });
        sg::apply_uniforms(1, &sg::Range { ptr: ((&self.modulate) as *const Vec4).cast(), size: 16 });

        sg::draw(0, 6, 1);
        self.renderer.draw_calls += 1;

        mvp

    }
}
