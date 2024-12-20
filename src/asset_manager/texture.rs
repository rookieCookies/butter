use derive_macros::Builder;
use serde::{Deserialize, Serialize};
use sokol::gfx::{self as sg, ImageData};
use tracing::{info, trace};

use crate::{clamp_to_i32, to_cstring};

use super::{AssetManager, TextureId};

#[derive(Debug, Deserialize, Serialize)]
pub struct Texture {
    image: u32,
    pub(super) texture_load_type: TextureLoadType,
}


#[derive(Debug, Deserialize, Serialize)]
pub enum TextureLoadType {
    Image(String),
    Script(String),
    Runtime,
}


#[derive(Builder)]
pub struct TextureBuilder {
    colour_format: ColourFormat,
    render_target: bool,
    width: usize,
    height: usize,
    usage: TextureUsage,
    sample_count: usize,
    #[ignore]
    data : Box<[u8]>,
    #[ignore]
    label: String,
}


///
/// A pixelformat name consist of three parts:
///   - components (R, RG, RGB or RGBA)
///   - bit width per component (8, 16 or 32)
///   - component data type:
///       - unsigned normalized (no postfix)
///       - signed normalized (SN postfix)
///       - unsigned integer (UI postfix)
///       - signed integer (SI postfix)
///       - float (F postfix)
///
/// Not all colour formats support the same things.
/// Call `ColourFormat::info()` on the format to see
/// what it supports.
///
#[derive(Clone, Copy, Debug, Default)]
pub enum ColourFormat {
    None,

    #[default]
    BGRA8,

    RGB8UI,
    RGBA8UI,

    RGB16UI,
    RGBA16UI,

    RGBA32UI,
    RGBA32F,
}


#[derive(Clone, Copy, Debug)]
pub struct ColourFormatInfo {
    pub sample: bool,
    pub filter: bool,
    pub render: bool,
    pub blend: bool,
    pub msaa: bool,
    pub depth: bool,
    pub compressed: bool,
    pub bytes_per_pixel: i32,
}


#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum TextureUsage {
    ///
    /// the resource will never be updated with
    /// new data, instead the content of the
    /// resource must be provided on creation
    ///
    #[default]
    Immutable,

    ///
    /// the resource will be updated infrequently
    /// with new data (this could range from "once
    /// after creation", to "quite often but not
    /// every frame")
    ///
    Dynamic,

    /// 
    /// the resource will be updated each frame
    /// with new content
    ///
    Stream,

}


impl TextureBuilder {
    pub fn new() -> Self {
        trace!("creating a new texture builder");
        Self {
            colour_format: ColourFormat::default(),
            render_target: false,
            width: 0,
            height: 0,
            usage: TextureUsage::default(),
            sample_count: 0,
            data: vec![].into(),
            label: String::from("undeclared"),
        }
    }
     


    pub fn label(mut self, label: &str) -> Self {
        trace!("texture builder name '{}' -> '{label}'", self.label);
        self.label = label.to_string();
        self
    }


    pub fn data(mut self, data: Box<[u8]>) -> Self {
        trace!("updated the data of the texture builder named '{}'", self.label);
        self.data = data;
        self
    }


    pub fn build(self, asset_manager: &mut AssetManager) -> TextureId {
        info!("creating a new texture named '{}'", self.label);
        info!("- colour_format: {:?}", self.colour_format);
        info!("- is_render_target: {}", self.render_target);
        info!("- width: {}", self.width);
        info!("- height: {}", self.height);
        info!("- usage: {:?}", self.usage);
        info!("- sample_count: {}", self.sample_count);
        info!("- len(data): {}", self.data.len());


        if self.usage == TextureUsage::Immutable {
            let pixel = self.colour_format.info();
            assert_eq!(self.data.len(), self.width * self.height * pixel.bytes_per_pixel as usize,
                    "texture usage pattern `immutable` requires the texture data \
                    to be initialised at the start. but `data.len()`({}) != `width * height * bytes_per_pixel`({}x{}x{} = {})",
                    self.data.len(), self.width, self.height, pixel.bytes_per_pixel, self.width * self.height * pixel.bytes_per_pixel as usize);
        }

        let mut image_data = ImageData::new();
        image_data.subimage[0][0] = sg::Range {
            ptr: self.data.as_ptr().cast(),
            size: self.data.len(),
        };

        let label = to_cstring("texture label", self.label);
        let image_desc = sg::ImageDesc {
            _type: sg::ImageType::Dim2,
            render_target: self.render_target,
            width: clamp_to_i32("texture width", self.width),
            height: clamp_to_i32("texture height", self.height),
            usage: match self.usage {
                TextureUsage::Immutable => sg::Usage::Immutable,
                TextureUsage::Dynamic => sg::Usage::Dynamic,
                TextureUsage::Stream => sg::Usage::Stream,
            },
            pixel_format: self.colour_format.to_sokol(),
            sample_count: clamp_to_i32("texture sample count", self.sample_count),
            data: image_data,
            label: label.as_ptr(),
            ..Default::default()
        };

        let image = sg::make_image(&image_desc);

        asset_manager.textures.push(Texture {
            image: image.id,
            texture_load_type: TextureLoadType::Runtime,
        })
    }
}


impl Texture {
    pub fn inner(&self) -> sg::Image {
        sg::Image{ id: self.image }
    }


    pub fn load_type(&self) -> &TextureLoadType {
        &self.texture_load_type
    }

}


impl ColourFormat {
    fn to_sokol(self) -> sg::PixelFormat {
        match self {
            ColourFormat::None => sg::PixelFormat::None,
            ColourFormat::BGRA8 => sg::PixelFormat::Bgra8,
            ColourFormat::RGB8UI => sg::PixelFormat::Rgba8ui,
            ColourFormat::RGBA8UI => sg::PixelFormat::Rgba8ui,
            ColourFormat::RGB16UI => sg::PixelFormat::Rgba16ui,
            ColourFormat::RGBA16UI => sg::PixelFormat::Rgba16ui,
            ColourFormat::RGBA32UI => sg::PixelFormat::Rgba32ui,
            ColourFormat::RGBA32F => sg::PixelFormat::Rgba32f,
        }
    }


    pub fn info(self) -> ColourFormatInfo {
        let info = sg::query_pixelformat(self.to_sokol());

        ColourFormatInfo {
            sample: info.sample,
            filter: info.filter,
            render: info.render,
            blend: info.blend,
            msaa: info.msaa,
            depth: info.depth,
            compressed: info.compressed,
            bytes_per_pixel: info.bytes_per_pixel,
        }
    }
}
