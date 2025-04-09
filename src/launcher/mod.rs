// In launcher/mod.rs
pub mod config;
mod process;

// Correct imports
pub use config::{update_jvm_args, get_jvm_args};
pub use config::get_minecraft_dir;
pub use process::launch_modpack;

// Import log macros
use log::{debug, warn};
