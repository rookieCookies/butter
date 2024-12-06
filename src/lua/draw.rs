#![allow(static_mut_refs)]
use std::marker::PhantomData;

use mlua::{Error, UserData};
use crate::{math::{matrix::{Matrix, Matrix4}, vector::{Vec2, Vec4}}, renderer::Renderer};


static mut DRAW : Option<&'static mut Renderer> = None;


pub struct Draw;


pub struct DrawRegistry<'a>(PhantomData<&'a ()>, Matrix4<f32>);


impl Draw {
    pub fn register<'a>(vp: Matrix<4, 4, f32>, bindings: &'a mut Renderer) -> DrawRegistry<'a> {
        // this lifetime transmute is alright, cos DrawRegistery will 
        // unregister it when the time calls for it
        let renderer = unsafe { core::mem::transmute::<&mut Renderer, &'static mut Renderer>(bindings) };

        let old_vp = renderer.vp;
        renderer.vp = vp;

        unsafe { DRAW = Some(renderer) };
        DrawRegistry(PhantomData, old_vp)
    }


    pub fn unregister() {
        unsafe { DRAW = None }
    }
}


impl<'me> Drop for DrawRegistry<'me> {
    fn drop(&mut self) {
        unsafe { DRAW.as_mut().unwrap().vp = self.1 };
        Draw::unregister();
    }
}


impl UserData for Draw {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("draw_quad", |_, (pos, scale, colour): (Vec2, Vec2, Vec4)| {
            if unsafe { DRAW.is_none() } {
                return Err(Error::runtime("draw calls are only accepted \
                                          in the 'draw' function of a component"))
            }

            unsafe { 
                DRAW.as_mut()
                    .unwrap()
                    .draw_quad()
                    .position(pos)
                    .scale(scale)
                    .modulate(colour)
                    .commit();
            };

            Ok(())
        });

    }
}
