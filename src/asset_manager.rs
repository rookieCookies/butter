pub mod texture;

use std::collections::HashMap;

use image::EncodableLayout;
use sti::{define_key, keyed::KVec};
use texture::{Texture, TextureBuilder, TextureLoadType};
use tracing::error;

use crate::engine::Engine;

define_key!(u32, pub TextureId);


#[derive(Debug)]
pub struct AssetManager {
    textures: KVec<TextureId, Texture>,
    path_to_texture: HashMap<String, TextureId>,
}


impl AssetManager {
    pub fn new() -> Self {
        Self {
            textures: KVec::new(),
            path_to_texture: HashMap::new(),
        }
    }


    pub fn init(&mut self) {
        let blank = TextureBuilder::new()
            .label("white")
            .width(1)
            .height(1)
            .data(Box::new([255; 4]))
            .colour_format(texture::ColourFormat::default())
            .build(self);

        assert_eq!(blank, TextureId::WHITE);
    }


    pub fn from_image(&mut self, path: &str) -> Option<TextureId> {
        if let Some(texture) = self.path_to_texture.get(path) { return Some(*texture) }

        let Ok(img) = image::ImageReader::open(path)
        else { error!("unable to read image at '{path}'"); return None };

        let Ok(img) = img.decode()
        else { error!("image at '{path}' is an unsupported format"); return None };

        let image = img.into_rgba32f();
        let texture = texture::TextureBuilder::new()
            .label(path)
            .width(image.width() as usize)
            .height(image.height() as usize)
            .colour_format(texture::ColourFormat::RGBA32F)
            .data(image.to_vec().as_bytes().to_vec().into_boxed_slice())
            .build(self);

        self.textures.get_mut(texture).unwrap().texture_load_type = TextureLoadType::Image(path.to_string());
        self.path_to_texture.insert(path.to_string(), texture);

        Some(texture)
    }


    pub fn from_script(&mut self, path: &str) -> Option<TextureId> {
        let mut script_manager = Engine::get().script_manager.borrow_mut();
        let script = script_manager.load_script(path);
        let script = script_manager.script(script);

        let texture = script.functions.texture(script.path())?;

        Some(texture)
    }


    pub fn texture(&self, script: TextureId) -> &Texture {
        &self.textures[script]
    }
}


impl TextureId {
    pub const WHITE : Self = Self(0);
}
