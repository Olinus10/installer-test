// Let's update the process.rs file to better handle different launcher types

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


// Launch Minecraft with a specific profile
pub fn launch_modpack(profile_id: &str) -> Result<(), String> {
    // Get the current launcher type
    let launcher_type = get_current_launcher_type()?;
    
    debug!("Launching modpack {} with {} launcher", profile_id, launcher_type);
    debug!("Profile ID being used: {}", profile_id);
    
    match launcher_type {
        LauncherType::Vanilla => launch_vanilla(profile_id),
        LauncherType::MultiMC => launch_multimc(profile_id),
        LauncherType::PrismLauncher => launch_prism(profile_id),
        LauncherType::Custom(path) => launch_custom_multimc(profile_id, path),
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

// Update launcher_profiles.json to set the profile as selected
fn update_launcher_profiles(profile_id: &str, minecraft_dir: &std::path::Path) -> Result<(), String> {
    let profiles_path = minecraft_dir.join("launcher_profiles.json");
    
    // Read the profiles file
    let content = match std::fs::read_to_string(&profiles_path) {
        Ok(content) => content,
        Err(e) => return Err(format!("Failed to read launcher profiles: {}", e))
    };
    
    // Parse it as JSON
    let mut profiles_json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(json) => json,
        Err(e) => return Err(format!("Failed to parse launcher profiles: {}", e))
    };
    
    // Update the profile's lastUsed field with current time
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
    
    // Write the modified file back
    match serde_json::to_string_pretty(&profiles_json) {
        Ok(updated_json) => {
            match std::fs::write(&profiles_path, updated_json) {
                Ok(_) => {
                    debug!("Successfully updated launcher profiles");
                    Ok(())
                },
                Err(e) => Err(format!("Failed to write updated launcher profiles: {}", e))
            }
        },
        Err(e) => Err(format!("Failed to serialize launcher profiles: {}", e))
    }
}

// Helper function to find Java executable
fn find_java_executable() -> Result<String, String> {
    // First look in standard locations
    let potential_paths = if cfg!(target_os = "windows") {
        vec![
            // Check bundled Java with Minecraft first
            format!("{}\\runtime\\java-runtime-gamma\\bin\\javaw.exe", 
                    crate::launcher::config::get_minecraft_dir().display()),
            format!("{}\\runtime\\jre-x64\\bin\\javaw.exe", 
                    crate::launcher::config::get_minecraft_dir().display()),
            // Then standard installation locations
            "C:\\Program Files\\Java\\jre-1.8\\bin\\javaw.exe".to_string(),
            "C:\\Program Files (x86)\\Java\\jre-1.8\\bin\\javaw.exe".to_string(),
            "C:\\Program Files\\Java\\jre1.8.0_301\\bin\\javaw.exe".to_string(),
            "C:\\Program Files (x86)\\Java\\jre1.8.0_301\\bin\\javaw.exe".to_string(),
            // Then try the latest Java versions
            "C:\\Program Files\\Java\\jre-latest\\bin\\javaw.exe".to_string(),
            "C:\\Program Files (x86)\\Java\\jre-latest\\bin\\javaw.exe".to_string(),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Library/Java/JavaVirtualMachines/jdk1.8.0_301.jdk/Contents/Home/bin/java".to_string(),
            "/Library/Java/JavaVirtualMachines/jdk-latest.jdk/Contents/Home/bin/java".to_string(),
        ]
    } else {
        vec![
            "/usr/bin/java".to_string(),
            "/usr/local/bin/java".to_string(),
        ]
    };
    
    // Check each path
    for path in potential_paths {
        if std::path::Path::new(&path).exists() {
            return Ok(path);
        }
    }
    
    // Try Java from system PATH
    if cfg!(target_os = "windows") {
        let output = match Command::new("where").arg("javaw.exe").output() {
            Ok(output) => output,
            Err(_) => return Err("Failed to locate Java executable".to_string())
        };
        
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            let first_line = path.lines().next().unwrap_or("");
            if !first_line.is_empty() {
                return Ok(first_line.to_string());
            }
        }
    } else {
        let output = match Command::new("which").arg("java").output() {
            Ok(output) => output,
            Err(_) => return Err("Failed to locate Java executable".to_string())
        };
        
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            let first_line = path.lines().next().unwrap_or("");
            if !first_line.is_empty() {
                return Ok(first_line.to_string());
            }
        }
    }
    
    Err("Could not find Java executable".to_string())
}

// Helper function to generate a random UUID
fn generate_random_uuid() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    // Format: 8-4-4-4-12 hex digits
    let uuid_parts = [
        format!("{:08x}", rng.gen::<u32>()),
        format!("{:04x}", rng.gen::<u16>()),
        format!("{:04x}", rng.gen::<u16>()),
        format!("{:04x}", rng.gen::<u16>()),
        format!("{:08x}{:04x}", rng.gen::<u32>(), rng.gen::<u16>()),
    ];
    
    uuid_parts.join("-")
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

// Find the Minecraft launcher executable - existing function, keep it
fn find_minecraft_launcher() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
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
