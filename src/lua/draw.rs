#![allow(static_mut_refs)]
use std::cell::Cell;

use mlua::{Error, UserData};
use crate::{engine::Engine, math::vector::{Vec2, Vec4}};


static mut DRAW : Cell<bool> = Cell::new(false);


pub struct Draw;


impl Draw {
    pub fn register<'a>() {
        unsafe { DRAW.set(true); };
    }


    pub fn unregister() {
        unsafe { DRAW.set(false); }
    }
}


impl UserData for Draw {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("draw_quad", |_, (pos, scale, colour): (Vec2, Vec2, Vec4)| {
            if unsafe { !DRAW.get() } {
                return Err(Error::runtime("draw calls are only accepted \
                                          in the 'draw' function of a component"))
            }

            Engine::generate().with(|engine| {
                engine
                    .renderer
                    .draw_quad()
                    .position(pos)
                    .scale(scale)
                    .modulate(colour)
                    .commit(&engine.asset_manager);
            });

            Ok(())
        });

    }
}
