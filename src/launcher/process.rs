use std::process::Command;
use log::{debug, error};

// Launch Minecraft with a specific profile
pub fn launch_modpack(profile_id: &str) -> Result<(), String> {
    // Find the Minecraft launcher executable
    let launcher_path = find_minecraft_launcher()?;
    
    debug!("Launching Minecraft with profile {}", profile_id);
    
    // Start the launcher process
    let mut command = Command::new(launcher_path);
    
    // Add arguments to select the profile
    command.arg("--workDir").arg(crate::launcher::config::get_minecraft_dir());
    
    // If your launcher can directly launch a specific profile, use these:
command.arg("--launch");
command.arg(profile_id);
    
    // Start the process
    match command.spawn() {
        Ok(_) => {
            debug!("Minecraft launcher started successfully");
            Ok(())
        },
        Err(e) => {
            error!("Failed to start Minecraft launcher: {}", e);
            Err(format!("Failed to start Minecraft launcher: {}", e))
        }
    }
}

// Find the Minecraft launcher executable
fn find_minecraft_launcher() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // Replace the existing code with this version that includes logging:
        debug!("Searching for Minecraft launcher on Windows...");
        
        // First try Program Files (x86)
        let program_files_x86 = match std::env::var("ProgramFiles(x86)") {
            Ok(path) => {
                debug!("Found Program Files (x86): {}", path);
                path
            },
            Err(e) => {
                debug!("Couldn't get Program Files (x86): {}", e);
                // Continue to try regular Program Files
                String::new()
            }
        };
        
        // Try regular Program Files if needed
        let program_files = if program_files_x86.is_empty() {
            match std::env::var("ProgramFiles") {
                Ok(path) => {
                    debug!("Found Program Files: {}", path);
                    path
                },
                Err(e) => {
                    debug!("Couldn't get Program Files either: {}", e);
                    return Err("Could not find Program Files directory".to_string());
                }
            }
        } else {
            program_files_x86
        };
        
        // Try the old path format
        let old_launcher_path = format!("{}\\Minecraft Launcher\\MinecraftLauncher.exe", program_files);
        debug!("Checking old launcher path: {}", old_launcher_path);
        
        if std::path::Path::new(&old_launcher_path).exists() {
            debug!("Found launcher at old path: {}", old_launcher_path);
            return Ok(old_launcher_path);
        }
        
        // Try the new path format
        let new_program_files = std::env::var("ProgramFiles").unwrap_or_default();
        let new_launcher_path = format!("{}\\Minecraft\\MinecraftLauncher.exe", new_program_files);
        debug!("Checking new launcher path: {}", new_launcher_path);
        
        if std::path::Path::new(&new_launcher_path).exists() {
            debug!("Found launcher at new path: {}", new_launcher_path);
            return Ok(new_launcher_path);
        }
        
        // Try Microsoft Store location
        let appdata = match std::env::var("LOCALAPPDATA") {
            Ok(path) => {
                debug!("Found LocalAppData: {}", path);
                path
            },
            Err(e) => {
                debug!("Couldn't get LocalAppData: {}", e);
                // Skip this check
                String::new()
            }
        };
        
        if !appdata.is_empty() {
            let ms_store_path = format!("{}\\Packages\\Microsoft.4297127D64EC6_8wekyb3d8bbwe\\LocalCache\\Local\\runtime\\jre-x64\\bin\\javaw.exe", appdata);
            debug!("Checking Microsoft Store path: {}", ms_store_path);
            
            if std::path::Path::new(&ms_store_path).exists() {
                debug!("Found launcher at MS Store path: {}", ms_store_path);
                return Ok(ms_store_path);
            }
        }
        
        // If you have Minecraft installed, check where it's located and add that path here
        debug!("No Minecraft launcher found at any expected locations");
        Err("Could not find Minecraft launcher".to_string())
    }
    
    #[cfg(target_os = "macos")]
    {
        let launcher_path = "/Applications/Minecraft.app/Contents/MacOS/launcher";
        if std::path::Path::new(launcher_path).exists() {
            return Ok(launcher_path.to_string());
        }
        Err("Could not find Minecraft launcher".to_string())
    }
    
    #[cfg(target_os = "linux")]
    {
        // Check common Linux locations
        let possible_paths = [
            "/usr/bin/minecraft-launcher",
            "/usr/local/bin/minecraft-launcher",
            "/opt/minecraft-launcher/minecraft-launcher"
        ];
        
        for path in possible_paths {
            if std::path::Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }
        
        Err("Could not find Minecraft launcher".to_string())
    }
}
