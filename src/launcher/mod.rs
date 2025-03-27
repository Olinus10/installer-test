// Enhanced launcher module with direct Minecraft launching capability
mod config;
mod process;
pub mod microsoft_auth; // Make the microsoft_auth module public

// Correct imports
pub use config::{update_jvm_args, get_jvm_args};

// Import log macros
use log::{debug, warn};

// Use our public Microsoft Auth module
pub use microsoft_auth::MicrosoftAuth;

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
