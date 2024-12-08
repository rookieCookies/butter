pub mod engine_version;

use engine_version::EngineVersion;
use tracing::info;
use serde::{Deserialize, Serialize};

use crate::math::vector::Vec2;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProjectSettings {
    pub engine: EngineSettings,
    pub window: WindowSettings,
    pub world : WorldSettings,
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EngineSettings {
    pub version: EngineVersion,
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WindowSettings {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default = "default_width")]
    pub width: usize,
    #[serde(default = "default_height")]
    pub height: usize,
    #[serde(default = "default_msaa_sample_count")]
    pub msaa_sample_count: usize,
    #[serde(default)]
    pub high_dpi: bool,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default)]
    pub allow_transparency: bool,
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WorldSettings {
    pub entry_scene: String,
    #[serde(default = "default_gravity")]
    pub gravity: Vec2,
    #[serde(default = "default_physics_framerate")]
    pub physics_framerate: usize,
}


impl ProjectSettings {
    pub fn new(file: &str) -> Result<Self, toml::de::Error> {
        info!("parsing project settings");
        let settings : Self = toml::from_str(file)?;

        info!("project settings:");
        info!("- engine.version: '{}' (current: '{}')", settings.engine.version, EngineVersion::CURRENT);
        info!("- window.title: '{}'", settings.window.title);
        info!("- window.width: {}", settings.window.width);
        info!("- window.height: {}", settings.window.height);
        info!("- window.msaa_sample_count: {}", settings.window.msaa_sample_count);
        info!("- window.high_dpi: {}", settings.window.high_dpi);
        info!("- window.fullscreen: {}", settings.window.fullscreen);
        info!("- window.allow_transparency: {}", settings.window.allow_transparency);
        info!("- world.entry_scene: {}", settings.world.entry_scene);
        Ok(settings)
    }
}


impl core::default::Default for ProjectSettings {
    fn default() -> Self {
        Self {
            engine: EngineSettings { version: EngineVersion::CURRENT },
            window: WindowSettings {
                title: "untitled project".to_string(),
                width: 800,
                height: 600,
                msaa_sample_count: 4,
                high_dpi: false,
                fullscreen: false,
                allow_transparency: true,
            },
            world: WorldSettings {
                entry_scene: String::new(),
                gravity: Vec2::new(0.0, -9.8),
                physics_framerate: 240,
            },
        }
    }
}


fn default_height() -> usize {
    600
}


fn default_width() -> usize {
    800
}


fn default_physics_framerate() -> usize {
    240
}


fn default_title() -> String {
    String::from("butter game")
}


fn default_gravity() -> Vec2 {
    Vec2::new(0.0, -9.8)
}


fn default_msaa_sample_count() -> usize {
    4
}
