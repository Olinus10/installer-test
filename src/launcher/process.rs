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
    let program_files = std::env::var("ProgramFiles(x86)")
        .or_else(|_| std::env::var("ProgramFiles"))
        .map_err(|_| "Could not find Program Files directory".to_string())?;
        
    // Try the old location
    let old_launcher_path = format!("{}\\Minecraft Launcher\\MinecraftLauncher.exe", program_files);
    if std::path::Path::new(&old_launcher_path).exists() {
        return Ok(old_launcher_path);
    }
    
    // Try the new location
    let program_files_no_x86 = std::env::var("ProgramFiles")
        .map_err(|_| "Could not find Program Files directory".to_string())?;
    let new_launcher_path = format!("{}\\Minecraft\\MinecraftLauncher.exe", program_files_no_x86);
    if std::path::Path::new(&new_launcher_path).exists() {
        return Ok(new_launcher_path);
    }
    
    // Try the Microsoft Store location
    let appdata = std::env::var("LOCALAPPDATA")
        .map_err(|_| "Could not find AppData directory".to_string())?;
    let ms_store_path = format!("{}\\Packages\\Microsoft.4297127D64EC6_8wekyb3d8bbwe\\LocalCache\\Local\\runtime\\jre-x64\\bin\\javaw.exe", appdata);
    if std::path::Path::new(&ms_store_path).exists() {
        return Ok(ms_store_path);
    }
    
    Err("Could not find Minecraft launcher. Please ensure it is installed.".to_string())
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
