mod config;
mod process;

pub use config::{update_jvm_args, get_jvm_args};
pub use config::get_minecraft_dir;

// Launch a modpack using the existing process implementation
pub fn launch_modpack(uuid: &str) -> Result<(), String> {
    process::launch_modpack(uuid)
}
