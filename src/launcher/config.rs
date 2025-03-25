use std::path::{Path, PathBuf};
use std::fs;
use serde_json::{Value, json};
use log::{error, debug};

// Default optimized JVM args
pub const DEFAULT_JVM_ARGS: &str = "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M -Xmx4G";

// Get the Minecraft directory
pub fn get_minecraft_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    
    #[cfg(target_os = "windows")]
    let mc_dir = home.join("AppData").join("Roaming").join(".minecraft");
    
    #[cfg(target_os = "macos")]
    let mc_dir = home.join("Library").join("Application Support").join("minecraft");
    
    #[cfg(target_os = "linux")]
    let mc_dir = home.join(".minecraft");
    
    mc_dir
}

// Get launcher_profiles.json path
fn get_profiles_path() -> PathBuf {
    get_minecraft_dir().join("launcher_profiles.json")
}

// Read the current launcher profiles
pub fn read_profiles() -> Result<Value, String> {
    let path = get_profiles_path();
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read launcher profiles: {}", e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse launcher profiles: {}", e))
}

// Update the JVM arguments for a specific profile
pub fn update_jvm_args(profile_id: &str, jvm_args: &str) -> Result<(), String> {
    let mut profiles = read_profiles()?;
    
    // Ensure the profiles object exists
    if !profiles["profiles"].is_object() {
        return Err("Invalid launcher_profiles.json format".into());
    }
    
    // Update or create the profile
    let profile_exists = profiles["profiles"].as_object()
        .unwrap()
        .contains_key(profile_id);
    
    if profile_exists {
        profiles["profiles"][profile_id]["javaArgs"] = json!(jvm_args);
    } else {
        // Profile doesn't exist - create it
        let new_profile = json!({
            "name": format!("Wynncraft Overhaul - {}", profile_id),
            "type": "custom",
            "created": chrono::Utc::now().to_rfc3339(),
            "lastUsed": chrono::Utc::now().to_rfc3339(),
            "icon": "Furnace",
            "javaArgs": jvm_args,
            "gameDir": format!("{}", get_minecraft_dir().join(format!("instances/{}", profile_id)).display())
        });
        
        if let Some(profiles_obj) = profiles["profiles"].as_object_mut() {
            profiles_obj.insert(profile_id.to_string(), new_profile);
        }
    }
    
    // Write the updated profiles back to the file
    let json_str = serde_json::to_string_pretty(&profiles)
        .map_err(|e| format!("Failed to serialize profiles: {}", e))?;
    
    fs::write(get_profiles_path(), json_str)
        .map_err(|e| format!("Failed to write launcher profiles: {}", e))?;
    
    debug!("Updated JVM args for profile {}: {}", profile_id, jvm_args);
    Ok(())
}

// Get current JVM args for a profile (returns default if not found)
pub fn get_jvm_args(profile_id: &str) -> Result<String, String> {
    // First check if we have custom arguments in our own config
    let app_data = crate::get_app_data();
    let custom_args_path = app_data.join(format!(".WC_OVHL/{}/jvm_args.txt", profile_id));
    
    if custom_args_path.exists() {
        match fs::read_to_string(&custom_args_path) {
            Ok(args) if !args.trim().is_empty() => {
                debug!("Using custom JVM args from {}: {}", custom_args_path.display(), args);
                return Ok(args);
            },
            _ => {
                debug!("Custom JVM args file exists but is empty or unreadable");
            }
        }
    }
    
    // Check for JVM args in the manifest
    let manifest_path = app_data.join(format!(".WC_OVHL/{}/manifest.json", profile_id));
    if manifest_path.exists() {
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<Value>(&content) {
                // Check for manifest-specified JVM args
                if let Some(java_args) = manifest["java_args"].as_str() {
                    debug!("Using JVM args from manifest: {}", java_args);
                    return Ok(java_args.to_string());
                }
                
                // Check for max_mem and min_mem
                let mut args = String::new();
                
                // Base optimization flags
                args.push_str("-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M");
                
                if let Some(max_mem) = manifest["max_mem"].as_i64() {
                    args.push_str(&format!(" -Xmx{}M", max_mem));
                } else {
                    args.push_str(" -Xmx4G");
                }
                
                if let Some(min_mem) = manifest["min_mem"].as_i64() {
                    args.push_str(&format!(" -Xms{}M", min_mem));
                }
                
                debug!("Generated JVM args from manifest settings: {}", args);
                return Ok(args);
            }
        }
    }
    
    // Try checking in launcher_profiles.json
    let profiles = match read_profiles() {
        Ok(profiles) => profiles,
        Err(_) => return Ok(DEFAULT_JVM_ARGS.to_string()),
    };
    
    if let Some(profile) = profiles["profiles"].get(profile_id) {
        if let Some(args) = profile["javaArgs"].as_str() {
            debug!("Found JVM args in launcher profile: {}", args);
            return Ok(args.to_string());
        }
    }
    
    // Return default if not found
    debug!("No specific JVM args found, using default: {}", DEFAULT_JVM_ARGS);
    Ok(DEFAULT_JVM_ARGS.to_string())
}
