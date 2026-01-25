use std::path::{Path, PathBuf};
use log::{debug, info, warn};

/// Find the Minecraft launcher executable on Windows
#[cfg(target_os = "windows")]
pub fn find_minecraft_launcher() -> Option<PathBuf> {
    // Common installation paths for Minecraft launcher
    let search_paths = vec![
        // Standard Program Files locations
        "C:\\Program Files (x86)\\Minecraft Launcher\\MinecraftLauncher.exe",
        "C:\\Program Files\\Minecraft Launcher\\MinecraftLauncher.exe",
        
        // Microsoft Store version
        "C:\\Program Files\\WindowsApps\\Microsoft.MinecraftLauncher_1.0.0.0_x64__8wekyb3d8bbwe\\Minecraft.exe",
        
        // Xbox Game Pass locations
        "C:\\XboxGames\\Minecraft Launcher\\Content\\MinecraftLauncher.exe",
        "C:\\Program Files\\XboxGames\\Minecraft Launcher\\Content\\MinecraftLauncher.exe",
        
        // Legacy locations
        "C:\\Program Files (x86)\\Minecraft\\MinecraftLauncher.exe",
        "C:\\Program Files\\Minecraft\\MinecraftLauncher.exe",
    ];
    
    // First, try the standard paths
    for path_str in &search_paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            debug!("Found Minecraft launcher at: {}", path.display());
            return Some(path);
        }
    }
    
    // Try to find via registry (Windows only)
    if let Some(path) = find_launcher_via_registry() {
        return Some(path);
    }
    
    // Search in user's home directory (Downloads, Desktop, Documents)
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let user_dir = PathBuf::from(user_profile);
        
        let user_search_paths = vec![
            user_dir.join("Downloads\\MinecraftLauncher.exe"),
            user_dir.join("Desktop\\MinecraftLauncher.exe"),
            user_dir.join("Documents\\MinecraftLauncher.exe"),
            user_dir.join("Downloads\\Minecraft Launcher\\MinecraftLauncher.exe"),
            user_dir.join("Desktop\\Minecraft Launcher\\MinecraftLauncher.exe"),
            user_dir.join("AppData\\Local\\Packages\\Microsoft.4297127D64EC6_8wekyb3d8bbwe\\LocalCache\\Local\\Minecraft\\MinecraftLauncher.exe"),
            
            // Xbox Game Pass user-specific locations
            user_dir.join("AppData\\Local\\Packages\\Microsoft.MinecraftUWP_8wekyb3d8bbwe\\LocalState\\MinecraftLauncher.exe"),
        ];
        
        for path in user_search_paths {
            if path.exists() {
                debug!("Found Minecraft launcher in user directory: {}", path.display());
                return Some(path);
            }
        }
    }
    
    // Last resort: search common drive letters
    for drive in &['C', 'D', 'E'] {
        if let Some(path) = search_drive_for_launcher(*drive) {
            return Some(path);
        }
    }
    
    warn!("Could not find Minecraft launcher in any common location");
    None
}

/// Search a specific drive for the Minecraft launcher (limited depth to avoid long searches)
#[cfg(target_os = "windows")]
fn search_drive_for_launcher(drive: char) -> Option<PathBuf> {
    use std::fs;
    
    let root = PathBuf::from(format!("{}:\\", drive));
    if !root.exists() {
        return None;
    }
    
    // Search in common directories to keep search fast
    let search_dirs = vec![
        root.join("Program Files (x86)"),
        root.join("Program Files"),
        root.join("XboxGames"),  // Xbox Game Pass installations
    ];
    
    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }
        
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.to_lowercase().contains("minecraft") {
                        // Check for launcher in this directory
                        let launcher_path = path.join("MinecraftLauncher.exe");
                        if launcher_path.exists() {
                            debug!("Found Minecraft launcher at: {}", launcher_path.display());
                            return Some(launcher_path);
                        }
                        
                        // Also check Xbox Game Pass "Content" subfolder
                        let content_launcher = path.join("Content\\MinecraftLauncher.exe");
                        if content_launcher.exists() {
                            debug!("Found Minecraft launcher in Content folder: {}", content_launcher.display());
                            return Some(content_launcher);
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Try to find the launcher via Windows registry
#[cfg(target_os = "windows")]
fn find_launcher_via_registry() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;
    
    // Try to read from registry where Minecraft might store its installation path
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    
    let registry_paths = vec![
        "SOFTWARE\\Mojang\\InstalledProducts\\Minecraft Launcher",
        "SOFTWARE\\WOW6432Node\\Mojang\\InstalledProducts\\Minecraft Launcher",
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Minecraft Launcher",
    ];
    
    for reg_path in registry_paths {
        if let Ok(key) = hklm.open_subkey(reg_path) {
            // Try to read InstallLocation or similar keys
            for value_name in &["InstallLocation", "InstallPath", "Path", ""] {
                if let Ok(install_path) = key.get_value::<String, _>(value_name) {
                    let launcher_path = PathBuf::from(install_path).join("MinecraftLauncher.exe");
                    if launcher_path.exists() {
                        debug!("Found Minecraft launcher via registry: {}", launcher_path.display());
                        return Some(launcher_path);
                    }
                }
            }
        }
    }
    
    None
}

/// macOS implementation
#[cfg(target_os = "macos")]
pub fn find_minecraft_launcher() -> Option<PathBuf> {
    let search_paths = vec![
        "/Applications/Minecraft.app/Contents/MacOS/launcher",
        "/Applications/Minecraft.app",
    ];
    
    for path_str in &search_paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            debug!("Found Minecraft launcher at: {}", path.display());
            return Some(path);
        }
    }
    
    // Check user's Applications folder
    if let Some(home) = dirs::home_dir() {
        let user_apps = home.join("Applications/Minecraft.app/Contents/MacOS/launcher");
        if user_apps.exists() {
            debug!("Found Minecraft launcher in user Applications: {}", user_apps.display());
            return Some(user_apps);
        }
    }
    
    warn!("Could not find Minecraft launcher on macOS");
    None
}

/// Linux implementation
#[cfg(target_os = "linux")]
pub fn find_minecraft_launcher() -> Option<PathBuf> {
    let search_paths = vec![
        "/usr/bin/minecraft-launcher",
        "/usr/local/bin/minecraft-launcher",
        "/opt/minecraft-launcher/minecraft-launcher",
        "/snap/bin/minecraft-launcher",  // Snap package
        "/var/lib/flatpak/exports/bin/com.mojang.Minecraft",  // Flatpak
    ];
    
    for path_str in &search_paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            debug!("Found Minecraft launcher at: {}", path.display());
            return Some(path);
        }
    }
    
    // Check user's local bin
    if let Some(home) = dirs::home_dir() {
        let local_paths = vec![
            home.join(".local/bin/minecraft-launcher"),
            home.join("bin/minecraft-launcher"),
            home.join(".local/share/applications/minecraft-launcher"),
        ];
        
        for path in local_paths {
            if path.exists() {
                debug!("Found Minecraft launcher in user directory: {}", path.display());
                return Some(path);
            }
        }
    }
    
    warn!("Could not find Minecraft launcher on Linux");
    None
}

/// Get a cached or freshly searched launcher path
pub fn get_launcher_path() -> Result<PathBuf, String> {
    // Try to load from cache first
    if let Some(cached_path) = load_cached_launcher_path() {
        if cached_path.exists() {
            debug!("Using cached launcher path: {}", cached_path.display());
            return Ok(cached_path);
        } else {
            warn!("Cached launcher path no longer exists, searching again...");
        }
    }
    
    // Search for the launcher
    match find_minecraft_launcher() {
        Some(path) => {
            info!("Found Minecraft launcher at: {}", path.display());
            // Cache the path for future use
            save_launcher_path_cache(&path);
            Ok(path)
        },
        None => Err("Could not find Minecraft launcher. Please ensure Minecraft is installed, or manually launch it from the Minecraft launcher.".to_string())
    }
}

/// Save the launcher path to cache
fn save_launcher_path_cache(path: &Path) {
    let cache_file = crate::get_app_data().join(".WC_OVHL/launcher_path.txt");
    if let Err(e) = std::fs::write(&cache_file, path.to_string_lossy().as_bytes()) {
        warn!("Failed to cache launcher path: {}", e);
    }
}

/// Load the cached launcher path
fn load_cached_launcher_path() -> Option<PathBuf> {
    let cache_file = crate::get_app_data().join(".WC_OVHL/launcher_path.txt");
    if let Ok(content) = std::fs::read_to_string(&cache_file) {
        let path = PathBuf::from(content.trim());
        Some(path)
    } else {
        None
    }
}
