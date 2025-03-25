mod config;
mod process;
mod ms_auth;

pub use config::{update_jvm_args, get_jvm_args, DEFAULT_JVM_ARGS, get_minecraft_dir};
pub use process::launch_modpack_legacy;
pub use ms_auth::MicrosoftAuth;

// Launch a modpack with the appropriate method
pub async fn launch_modpack(uuid: &str) -> Result<(), String> {
    // First, try to launch with Microsoft authentication (direct launch)
    match MicrosoftAuth::launch_minecraft(uuid).await {
        Ok(_) => {
            log::info!("Successfully launched modpack {} using direct Microsoft authentication", uuid);
            return Ok(());
        },
        Err(e) => {
            log::warn!("Microsoft authentication launch failed: {}", e);
            log::info!("Falling back to legacy launcher method");
            
            // Fall back to the legacy method (Minecraft Launcher)
            match launch_modpack_legacy(uuid) {
                Ok(_) => {
                    log::info!("Successfully launched modpack {} using legacy launcher method", uuid);
                    Ok(())
                },
                Err(e) => {
                    log::error!("Failed to launch modpack {}: {}", uuid, e);
                    Err(format!("Failed to launch modpack: {}", e))
                }
            }
        }
    }
}
