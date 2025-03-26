use std::process::Command;
use log::{debug, error};
use std::fmt;

#[derive(Debug)]
enum LauncherType {
    Vanilla,
    MultiMC,
    PrismLauncher,
    Custom(String),
}

// Implement Display trait for LauncherType
impl fmt::Display for LauncherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LauncherType::Vanilla => write!(f, "Vanilla"),
            LauncherType::MultiMC => write!(f, "MultiMC"),
            LauncherType::PrismLauncher => write!(f, "Prism Launcher"),
            LauncherType::Custom(path) => write!(f, "Custom ({})", path),
        }
    }
}

// Main function to launch Minecraft with a specific profile
pub fn launch_modpack(profile_id: &str) -> Result<(), String> {
    // Instead of using the default launcher, use the Microsoft auth path
    match crate::launcher::MicrosoftAuth::launch_minecraft(profile_id) {
        Ok(_) => {
            debug!("Successfully launched modpack: {}", profile_id);
            Ok(())
        },
        Err(e) => {
            error!("Failed to launch modpack: {}", e);
            Err(format!("Failed to launch modpack: {}", e))
        }
    }
}

// Determine which launcher we're using
fn get_current_launcher_type() -> Result<LauncherType, String> {
    // Read config to determine current launcher
    let config = match std::fs::read_to_string(crate::get_app_data().join(".WC_OVHL/config.json")) {
        Ok(content) => content,
        Err(e) => return Err(format!("Failed to read config: {}", e)),
    };
    
    let config: serde_json::Value = match serde_json::from_str(&config) {
        Ok(parsed) => parsed,
        Err(e) => return Err(format!("Failed to parse config: {}", e)),
    };
    
    let launcher = match config["launcher"].as_str() {
        Some(val) => val,
        None => return Err("Launcher not specified in config".to_string()),
    };
    
    match launcher {
        "vanilla" => Ok(LauncherType::Vanilla),
        "multimc-MultiMC" => Ok(LauncherType::MultiMC),
        "multimc-PrismLauncher" => Ok(LauncherType::PrismLauncher),
        custom if custom.starts_with("custom-") => {
            let path = custom.trim_start_matches("custom-").to_string();
            Ok(LauncherType::Custom(path))
        },
        _ => Err(format!("Unknown launcher type: {}", launcher)),
    }
}

// Launch vanilla Minecraft with the specified profile
fn launch_vanilla(profile_id: &str) -> Result<(), String> {
    debug!("Setting up debug launcher for profile {}", profile_id);
    
    // Build the complete game directory path
    let minecraft_dir = crate::launcher::config::get_minecraft_dir();
    let game_dir = minecraft_dir.join(format!(".WC_OVHL/{}", profile_id));
    
    debug!("Game directory: {:?}", game_dir);
    
    // Create a debug batch file
    let script_path = std::env::temp_dir().join(format!("debug_minecraft_{}.bat", profile_id));
    
    // This batch file will:
    // 1. Show detailed debug information
    // 2. Keep the window open to display errors
    // 3. Try to update the launcher profiles
    // 4. Launch the Minecraft launcher
    let batch_content = format!(
        "@echo off\r\n\
         echo ===== MINECRAFT DEBUG LAUNCHER =====\r\n\
         echo Profile ID: {}\r\n\
         echo Game directory: {}\r\n\
         echo Minecraft directory: {}\r\n\
         echo ===================================\r\n\
         \r\n\
         echo Checking Java availability...\r\n\
         where java >nul 2>nul\r\n\
         if %ERRORLEVEL% NEQ 0 (\r\n\
             echo Java not found in PATH! Will try to use Minecraft bundled Java.\r\n\
         ) else (\r\n\
             echo Java found in PATH.\r\n\
             java -version\r\n\
         )\r\n\
         \r\n\
         echo Updating profile as most recently used...\r\n\
         \r\n\
         echo Launching Minecraft launcher...\r\n\
         start \"\" \"C:\\Program Files (x86)\\Minecraft Launcher\\MinecraftLauncher.exe\"\r\n\
         \r\n\
         echo The launcher has been started.\r\n\
         echo Your profile '{}' should be selected.\r\n\
         echo Just click the PLAY button in the launcher to start the game.\r\n\
         echo.\r\n\
         echo Press any key to close this window...\r\n\
         pause > nul\r\n",
        profile_id,
        game_dir.display(),
        minecraft_dir.display(),
        profile_id
    );
    
    // Update the launcher profiles to make this the default
    let profiles_path = minecraft_dir.join("launcher_profiles.json");
    
    if profiles_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&profiles_path) {
            if let Ok(mut profiles_json) = serde_json::from_str::<serde_json::Value>(&content) {
                // Update lastUsed timestamp
                let now = chrono::Utc::now().to_rfc3339();
                
                if let Some(profiles) = profiles_json.get_mut("profiles") {
                    if let Some(profile) = profiles.get_mut(profile_id) {
                        if let Some(profile_obj) = profile.as_object_mut() {
                            profile_obj.insert("lastUsed".to_string(), serde_json::Value::String(now));
                            debug!("Updated lastUsed timestamp for profile {}", profile_id);
                        }
                    }
                }
                
                // Set as the selected profile
                if let Some(launcher_version) = profiles_json.get_mut("launcherVersion") {
                    if let Some(launcher_obj) = launcher_version.as_object_mut() {
                        launcher_obj.insert("selectedProfileId".to_string(), 
                                        serde_json::Value::String(profile_id.to_string()));
                        debug!("Set profile {} as selected", profile_id);
                    }
                }
                
                // Write back the updated profiles
                if let Ok(updated_json) = serde_json::to_string_pretty(&profiles_json) {
                    let _ = std::fs::write(&profiles_path, updated_json);
                }
            }
        }
    }
    
    // Write the batch file
    match std::fs::write(&script_path, batch_content) {
        Ok(_) => debug!("Created debug launcher script at {:?}", script_path),
        Err(e) => return Err(format!("Failed to create debug launcher script: {}", e))
    }
    
    // Execute the batch file
    debug!("Running debug launcher script");
    match Command::new("cmd.exe")
        .arg("/C")
        .arg("start")
        .arg(script_path.to_str().unwrap())
        .spawn() 
    {
        Ok(_) => {
            debug!("Debug launcher script executed successfully");
            Ok(())
        },
        Err(e) => {
            error!("Failed to execute debug launcher script: {}", e);
            Err(format!("Failed to execute debug launcher script: {}", e))
        }
    }
}

// Launch MultiMC with the specified instance
fn launch_multimc(profile_id: &str) -> Result<(), String> {
    let multimc_path = crate::get_multimc_folder("MultiMC")
        .map_err(|e| format!("Failed to find MultiMC folder: {}", e))?;
    
    let executable = if cfg!(target_os = "windows") {
        multimc_path.join("MultiMC.exe")
    } else if cfg!(target_os = "macos") {
        multimc_path.join("MultiMC.app/Contents/MacOS/MultiMC")
    } else {
        multimc_path.join("MultiMC")
    };
    
    debug!("Launching MultiMC with instance {}", profile_id);
    debug!("MultiMC executable path: {:?}", executable);
    
    // Try to determine if the instance exists
    let instance_dir = multimc_path.join("instances").join(profile_id);
    if !instance_dir.exists() {
        debug!("Warning: Instance directory does not exist: {:?}", instance_dir);
    }
    
    // Launch MultiMC with the instance
    let command = Command::new(&executable)
        .arg("--launch").arg(profile_id) // Try different launch syntax
        .spawn();
        
    debug!("Command attempted: {:?} --launch {}", executable, profile_id);
    
    match command {
        Ok(_) => {
            debug!("MultiMC launched successfully with instance: {}", profile_id);
            Ok(())
        },
        Err(e) => {
            error!("Failed to start MultiMC: {}", e);
            Err(format!("Failed to start MultiMC: {}", e))
        }
    }
}

// Launch Prism Launcher with the specified instance
fn launch_prism(profile_id: &str) -> Result<(), String> {
    let prism_path = crate::get_multimc_folder("PrismLauncher")
        .map_err(|e| format!("Failed to find Prism Launcher folder: {}", e))?;
    
    let executable = if cfg!(target_os = "windows") {
        prism_path.join("prismlauncher.exe")
    } else if cfg!(target_os = "macos") {
        prism_path.join("prismlauncher.app/Contents/MacOS/prismlauncher")
    } else {
        prism_path.join("prismlauncher")
    };
    
    debug!("Launching Prism Launcher with instance {}", profile_id);
    
    // Launch Prism with the instance
    match Command::new(executable)
        .arg("-l") // Launch instance directly
        .arg(profile_id)
        .spawn() {
            Ok(_) => {
                debug!("Prism Launcher launched successfully with instance: {}", profile_id);
                Ok(())
            },
            Err(e) => {
                error!("Failed to start Prism Launcher: {}", e);
                Err(format!("Failed to start Prism Launcher: {}", e))
            }
        }
}

// Launch custom MultiMC with the specified instance
fn launch_custom_multimc(profile_id: &str, path: String) -> Result<(), String> {
    // Convert path string to PathBuf
    let custom_path = std::path::PathBuf::from(path);
    
    // Try to determine the executable name
    let executable = if cfg!(target_os = "windows") {
        // Look for common executables
        if custom_path.join("MultiMC.exe").exists() {
            custom_path.join("MultiMC.exe")
        } else if custom_path.join("prismlauncher.exe").exists() {
            custom_path.join("prismlauncher.exe")
        } else {
            // Default to MultiMC.exe and hope for the best
            custom_path.join("MultiMC.exe")
        }
    } else if cfg!(target_os = "macos") {
        // Look for common executables
        if custom_path.join("MultiMC.app/Contents/MacOS/MultiMC").exists() {
            custom_path.join("MultiMC.app/Contents/MacOS/MultiMC")
        } else if custom_path.join("prismlauncher.app/Contents/MacOS/prismlauncher").exists() {
            custom_path.join("prismlauncher.app/Contents/MacOS/prismlauncher")
        } else {
            // Default to MultiMC and hope for the best
            custom_path.join("MultiMC.app/Contents/MacOS/MultiMC")
        }
    } else {
        // Linux: look for common executables
        if custom_path.join("MultiMC").exists() {
            custom_path.join("MultiMC")
        } else if custom_path.join("prismlauncher").exists() {
            custom_path.join("prismlauncher")
        } else {
            // Default to MultiMC and hope for the best
            custom_path.join("MultiMC")
        }
    };
    
    debug!("Launching custom MultiMC-like launcher with instance {}", profile_id);
    
    // Launch with the instance
    match Command::new(executable)
        .arg("-l") // Launch instance directly
        .arg(profile_id)
        .spawn() {
            Ok(_) => {
                debug!("Custom launcher launched successfully with instance: {}", profile_id);
                Ok(())
            },
            Err(e) => {
                error!("Failed to start custom launcher: {}", e);
                Err(format!("Failed to start custom launcher: {}", e))
            }
        }
}
