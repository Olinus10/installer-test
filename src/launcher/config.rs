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
    let profiles = read_profiles()?;
    
    if let Some(profile) = profiles["profiles"].get(profile_id) {
        if let Some(args) = profile["javaArgs"].as_str() {
            return Ok(args.to_string());
        }
    }
    
    // Return default if not found
    Ok(DEFAULT_JVM_ARGS.to_string())
}
