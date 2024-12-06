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
    pub title: String,
    pub width: usize,
    pub height: usize,
    pub msaa_sample_count: usize,
    pub high_dpi: bool,
    pub fullscreen: bool,
    pub allow_transparency: bool,
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WorldSettings {
    pub entry_scene: String,
    pub gravity: Vec2,
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
            world: WorldSettings { entry_scene: String::new(), gravity: Vec2::new(0.0, -9.8) },
        }
    }
}
