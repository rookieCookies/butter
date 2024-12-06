pub mod time;
pub mod math;
pub mod input;
pub mod texture;
pub mod node;
pub mod physics_server;
pub mod draw;
pub mod scene;
pub mod engine;

use draw::Draw;
use input::Input;
use math::Math;
use mlua::{Lua, UserData};
use physics_server::Physics;
use scene::Scene;
use texture::LuaTexture;
use time::Time;
use tracing::{error, info};

pub fn setup_lua_environment(lua: &Lua) {
    info!("setting up lua environment");

    fn register<T: UserData + mlua::MaybeSend + 'static>(lua: &Lua, name: &str, data: T) {
        let Ok(userdata) = lua.create_userdata(data)
        else {
            error!("unable to create the '{name}' module");
            return;
        };

        if lua.globals().set(name, userdata).is_err() {
            error!("unable to register the '{name}' module");
            return;
        }
    }

    register(lua, "Time", Time);
    register(lua, "Math", Math);
    register(lua, "Input", Input);
    register(lua, "Texture", LuaTexture);
    register(lua, "PhysicsServer", Physics);
    register(lua, "Draw", Draw);
    register(lua, "SceneManager", Scene);
    register(lua, "Engine", engine::Engine);
}


