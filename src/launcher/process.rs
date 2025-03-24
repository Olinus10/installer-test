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
    debug!("Launching vanilla Minecraft with profile {}", profile_id);
    
    // Get the launcher executable path
    let launcher_path = find_minecraft_launcher()?;
    
    // Build the complete game directory path
    let minecraft_dir = crate::launcher::config::get_minecraft_dir();
    let game_dir = minecraft_dir.join(format!(".WC_OVHL/{}", profile_id));
    
    debug!("Game directory: {:?}", game_dir);
    
    // Try multiple different command lines to work with different launcher versions
    let launch_attempts = vec![
        // Approach 1: Direct approach with --launch parameter
        format!("\"{}\" --workDir \"{}\" --launch \"{}\"", 
            launcher_path, minecraft_dir.display(), profile_id),
            
        // Approach 2: Try with --profile instead of --launch
        format!("\"{}\" --workDir \"{}\" --profile \"{}\"",
            launcher_path, minecraft_dir.display(), profile_id),
            
        // Approach 3: Using gameDir and direct version specification
        format!("\"{}\" --gameDir \"{}\"",
            launcher_path, game_dir.display()),
            
        // Approach 4: Just launch the default profile
        format!("\"{}\"", launcher_path)
    ];
    
    // Create a temporary batch file
    let script_path = std::env::temp_dir().join(format!("launch_minecraft_{}.bat", profile_id));
    
    // Create a batch script that tries all approaches
    let mut batch_content = format!("@echo off\r\n");
    batch_content.push_str(&format!("echo Attempting to launch Minecraft with profile: {}\r\n\r\n", profile_id));
    
    // Add each launch attempt to the batch file
    for (i, attempt) in launch_attempts.iter().enumerate() {
        batch_content.push_str(&format!("echo Attempt {}...\r\n", i + 1));
        batch_content.push_str(&format!("{}\r\n", attempt));
        batch_content.push_str("if %ERRORLEVEL% EQU 0 goto success\r\n");
        batch_content.push_str("echo Attempt failed, trying next method...\r\n\r\n");
    }
    
    // Add success and failure handlers
    batch_content.push_str(":success\r\n");
    batch_content.push_str("echo Successfully launched Minecraft!\r\n");
    batch_content.push_str("exit /b 0\r\n");
    batch_content.push_str(":failure\r\n");
    batch_content.push_str("echo All launch attempts failed.\r\n");
    batch_content.push_str("pause\r\n");
    batch_content.push_str("exit /b 1\r\n");
    
    // Write batch file
    match std::fs::write(&script_path, batch_content) {
        Ok(_) => debug!("Created multi-approach launch script at {:?}", script_path),
        Err(e) => return Err(format!("Failed to create launch script: {}", e))
    }
    
    // Execute the batch file
    debug!("Running script: {:?}", script_path);
    match Command::new("cmd.exe").arg("/C").arg(&script_path).spawn() {
        Ok(_) => {
            debug!("Minecraft launch script executed successfully");
            Ok(())
        },
        Err(e) => {
            error!("Failed to execute Minecraft launch script: {}", e);
            Err(format!("Failed to execute Minecraft launch script: {}", e))
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
