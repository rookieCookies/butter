use tracing::trace;

use crate::{event_manager::{Event, Keycode}, math::vector::Vec3};

#[derive(Debug)]
pub struct InputManager {
    keys: [KeyState; 512],
    just_changed: Vec<Keycode>,
}


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum KeyState {
    Up,
    JustPressed,
    Down,
    JustReleased,
}


#[derive(Debug, Clone, Copy)]
pub enum KeyChange {
    Pressed,
    Released,
}


impl InputManager {
    pub fn new() -> Self {
        
        Self {
            keys: [KeyState::Up; 512],
            just_changed: vec![], 
        }
    }


    pub fn process<'a>(&mut self, events: impl Iterator<Item=&'a Event>) {
        trace!("processing input events");
        for key in self.just_changed.iter() {
            let key = &mut self.keys[*key as usize];
            let new_key = match key {
                KeyState::JustPressed => KeyState::Down,
                KeyState::JustReleased => KeyState::Up,


                  KeyState::Up
                | KeyState::Down => continue, 
            };

            *key = new_key;
        }

        self.just_changed.clear();

        for e in events {
            match e {
                Event::KeyUp(key, repeat) => {
                    if *repeat { continue }
                    self.keys[*key as usize] = KeyState::JustReleased;
                    self.just_changed.push(*key);
                }


                Event::KeyDown(key, repeat) => {
                    if *repeat { continue }
                    self.keys[*key as usize] = KeyState::JustPressed;
                    self.just_changed.push(*key);
                }


                _ => continue,
            }
        }
    }


    pub fn get_axis(&self, pos: Keycode, neg: Keycode) -> f32 {
        let mut power = 0.0;
        if self.is_key_down(pos) { power += 1.0 }
        if self.is_key_down(neg) { power -= 1.0 }
        power
    }


    pub fn get_vector(&self, pos_x: Keycode, neg_x: Keycode, pos_y: Keycode, neg_y: Keycode) -> Vec3 {
        Vec3::new(self.get_axis(pos_x, neg_x), self.get_axis(pos_y, neg_y), 0.0)
    }


    pub fn is_key_down(&self, key: Keycode) -> bool {
        let key = self.keys[key as usize];
        key == KeyState::JustPressed || key == KeyState::Down
    }


    pub fn is_key_up(&self, key: Keycode) -> bool {
        let key = self.keys[key as usize];
        key == KeyState::JustReleased || key == KeyState::Up
    }


    pub fn is_key_just_pressed(&self, key: Keycode) -> bool {
        let key = self.keys[key as usize];
        key == KeyState::JustPressed
    }


    pub fn is_key_just_released(&self, key: Keycode) -> bool {
        let key = self.keys[key as usize];
        key == KeyState::JustReleased
    }
}
