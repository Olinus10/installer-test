// In launcher/mod.rs
pub mod config; // Change to pub to allow access from microsoft_auth_impl.rs
mod process;

// Correct imports
pub use config::{update_jvm_args, get_jvm_args};
pub use config::get_minecraft_dir; // Explicitly re-export this function

// Import log macros
use log::{debug, warn};

// Launch a modpack using MS auth or fallback to the existing process implementation
pub fn launch_modpack(uuid: &str) -> Result<(), String> {
    // First try Microsoft auth to launch directly
    match MicrosoftAuth::launch_minecraft(uuid) {
        Ok(_) => {
            debug!("Successfully launched modpack via Microsoft auth: {}", uuid);
            Ok(())
        },
        Err(e) => {
            // Log the error
            warn!("Microsoft auth launch failed, falling back to process launcher: {}", e);
            
            // Fallback to the existing launch process
            process::launch_modpack(uuid)
        }
    }
}
