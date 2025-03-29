use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use log::{debug, error, info};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{CachedHttpClient, InstallerProfile, launcher};
use crate::preset::Preset;

// Structure for managing an installation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Installation {
    // Core identity properties
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    
    // Minecraft configuration
    pub minecraft_version: String,
    pub loader_type: String,      // "fabric", "quilt", etc.
    pub loader_version: String,
    
    // Path information
    pub installation_path: PathBuf,
    
    // Mod configuration
    pub enabled_features: Vec<String>,
    
    // Performance settings
    pub memory_allocation: i32,   // in MB
    pub java_args: String,
    
    // Account linking
    pub linked_account_id: Option<String>,
    
    // Installation status tracking
    pub installed: bool,
    pub modified: bool,           // True if changes were made but not yet applied
    pub update_available: bool,   // True if modpack has updates
    
    // Launcher and versioning information
    pub launcher_type: String,    // "vanilla", "multimc", etc.
    pub universal_version: String, // Which version of the universal modpack this uses
    
    // Last launch info for statistics
    pub last_launch: Option<DateTime<Utc>>,
    pub total_launches: u32,
}

// Struct for the installations index file
#[derive(Debug, Deserialize, Serialize)]
pub struct InstallationsIndex {
    pub installations: Vec<String>,  // List of installation IDs
    pub active_installation: Option<String>, // Currently selected installation
    pub last_active: Option<DateTime<Utc>>,
}

impl Default for InstallationsIndex {
    fn default() -> Self {
        Self {
            installations: Vec::new(),
            active_installation: None,
            last_active: Some(Utc::now()),
        }
    }
}

impl Installation {
    // Create a new installation from a preset
    pub fn new_from_preset(
        name: String,
        preset: &Preset,
        minecraft_version: String,
        loader_type: String,
        loader_version: String,
        launcher_type: String,
        universal_version: String,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        info!("Creating new installation '{}' with ID: {}", name, id);
        
        // Generate installation path based on ID
        let installation_path = get_installations_dir().join(&id);
        
        // Use preset's recommended settings or defaults
        let memory_allocation = preset.recommended_memory.unwrap_or(3072); // 3GB default
        let java_args = preset.recommended_java_args.clone().unwrap_or_else(|| 
            "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M".to_string()
        );
        
        Self {
            id,
            name,
            created_at: now,
            last_used: now,
            minecraft_version,
            loader_type,
            loader_version,
            installation_path,
            enabled_features: preset.enabled_features.clone(),
            memory_allocation,
            java_args,
            linked_account_id: None,
            installed: false,
            modified: false,
            update_available: false,
            launcher_type,
            universal_version,
            last_launch: None,
            total_launches: 0,
        }
    }
    
    // Custom installation without using a preset
    pub fn new_custom(
        name: String,
        minecraft_version: String,
        loader_type: String,
        loader_version: String,
        launcher_type: String,
        universal_version: String,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        info!("Creating new custom installation '{}' with ID: {}", name, id);
        
        // Generate installation path based on ID
        let installation_path = get_installations_dir().join(&id);
        
        Self {
            id,
            name,
            created_at: now,
            last_used: now,
            minecraft_version,
            loader_type,
            loader_version,
            installation_path,
            enabled_features: vec!["default".to_string()],
            memory_allocation: 3072, // 3GB default
            java_args: "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M".to_string(),
            linked_account_id: None,
            installed: false,
            modified: false,
            update_available: false,
            launcher_type,
            universal_version,
            last_launch: None,
            total_launches: 0,
        }
    }
    
    // Convert to an InstallerProfile for the actual installation process
    pub fn to_installer_profile(&self, http_client: &CachedHttpClient) -> InstallerProfile {
        // This implementation will depend on your InstallerProfile structure
        // You'll need to adapt this based on your existing code
        unimplemented!("Convert Installation to InstallerProfile")
    }
    
    // Install or update the installation
    pub async fn install_or_update(&mut self, http_client: &CachedHttpClient) -> Result<(), String> {
        if !self.installed || self.update_available || self.modified {
            info!("Installing/updating installation '{}'", self.name);
            
            // Convert to installer profile
            let installer_profile = self.to_installer_profile(http_client);
            
            // Perform installation
            if !self.installed {
                match crate::install(&installer_profile, || {}).await {
                    Ok(_) => {
                        self.installed = true;
                        self.modified = false;
                        self.update_available = false;
                        self.last_used = Utc::now();
                        info!("Successfully installed '{}'", self.name);
                        Ok(())
                    },
                    Err(e) => {
                        error!("Failed to install '{}': {}", self.name, e);
                        Err(e)
                    }
                }
            } else {
                // Update existing installation
                match crate::update(&installer_profile, || {}).await {
                    Ok(_) => {
                        self.modified = false;
                        self.update_available = false;
                        self.last_used = Utc::now();
                        info!("Successfully updated '{}'", self.name);
                        Ok(())
                    },
                    Err(e) => {
                        error!("Failed to update '{}': {}", self.name, e);
                        Err(e)
                    }
                }
            }
        } else {
            debug!("No changes needed for installation '{}'", self.name);
            Ok(())
        }
    }
    
    // Launch the installation
    pub fn launch(&mut self) -> Result<(), String> {
        info!("Launching installation '{}'", self.name);
        
        // Check if Microsoft auth is needed
        if crate::launcher::microsoft_auth::MicrosoftAuth::is_authenticated() {
            match crate::launcher::microsoft_auth::MicrosoftAuth::launch_minecraft(&self.id) {
                Ok(_) => {
                    // Update statistics
                    self.last_launch = Some(Utc::now());
                    self.total_launches += 1;
                    self.last_used = Utc::now();
                    self.save()?;
                    
                    info!("Successfully launched installation '{}'", self.name);
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to launch installation '{}': {}", self.name, e);
                    Err(format!("Failed to launch: {}", e))
                }
            }
        } else {
            // Need to authenticate first
            Err("Authentication required before launching. Please click 'Login with Microsoft' first.".to_string())
        }
    }
    
    // Save the installation to disk
    pub fn save(&self) -> Result<(), String> {
        let installations_dir = get_installations_dir();
        let installation_dir = installations_dir.join(&self.id);
        
        // Create directory if it doesn't exist
        if !installation_dir.exists() {
            fs::create_dir_all(&installation_dir)
                .map_err(|e| format!("Failed to create installation directory: {}", e))?;
        }
        
        // Save installation config
        let installation_json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize installation: {}", e))?;
        
        let config_path = installation_dir.join("installation.json");
        fs::write(&config_path, installation_json)
            .map_err(|e| format!("Failed to write installation config: {}", e))?;
        
        debug!("Saved installation '{}' to {}", self.name, config_path.display());
        
        Ok(())
    }
    
    // Delete the installation
    pub fn delete(&self) -> Result<(), String> {
        info!("Deleting installation '{}'", self.name);
        
        // Remove from index first
        let mut index = load_installations_index()
            .map_err(|e| format!("Failed to load installations index: {}", e))?;
        
        index.installations.retain(|id| id != &self.id);
        
        // If this was the active installation, clear that
        if let Some(active_id) = &index.active_installation {
            if active_id == &self.id {
                index.active_installation = None;
            }
        }
        
        // Save updated index
        save_installations_index(&index)
            .map_err(|e| format!("Failed to update installations index: {}", e))?;
        
        // Now remove the installation files
        let installation_dir = get_installations_dir().join(&self.id);
        if installation_dir.exists() {
            fs::remove_dir_all(&installation_dir)
                .map_err(|e| format!("Failed to delete installation directory: {}", e))?;
            
            info!("Successfully deleted installation '{}'", self.name);
        }
        
        Ok(())
    }
}

// Load an installation by ID
pub fn load_installation(id: &str) -> Result<Installation, String> {
    let installation_dir = get_installations_dir().join(id);
    let config_path = installation_dir.join("installation.json");
    
    if !config_path.exists() {
        return Err(format!("Installation {} not found", id));
    }
    
    let config_json = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read installation config: {}", e))?;
    
    let installation: Installation = serde_json::from_str(&config_json)
        .map_err(|e| format!("Failed to parse installation config: {}", e))?;
    
    Ok(installation)
}

// Load all installations
pub fn load_all_installations() -> Result<Vec<Installation>, String> {
    let index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    let mut installations = Vec::new();
    
    for id in &index.installations {
        match load_installation(id) {
            Ok(installation) => installations.push(installation),
            Err(e) => {
                error!("Failed to load installation {}: {}", id, e);
                // Continue loading other installations
            }
        }
    }
    
    // Sort by last used (most recent first)
    installations.sort_by(|a, b| b.last_used.cmp(&a.last_used));
    
    Ok(installations)
}

// Get the active installation
pub fn get_active_installation() -> Result<Option<Installation>, String> {
    let index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    if let Some(active_id) = &index.active_installation {
        match load_installation(active_id) {
            Ok(installation) => Ok(Some(installation)),
            Err(e) => {
                error!("Failed to load active installation: {}", e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

// Set the active installation
pub fn set_active_installation(id: &str) -> Result<(), String> {
    let mut index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    // Verify installation exists
    if !index.installations.contains(&id.to_string()) {
        return Err(format!("Installation {} not found", id));
    }
    
    index.active_installation = Some(id.to_string());
    index.last_active = Some(Utc::now());
    
    save_installations_index(&index)
}

// Load installations index
pub fn load_installations_index() -> Result<InstallationsIndex, io::Error> {
    let index_path = get_installations_dir().join("index.json");
    
    if !index_path.exists() {
        return Ok(InstallationsIndex::default());
    }
    
    let index_json = fs::read_to_string(index_path)?;
    let index: InstallationsIndex = serde_json::from_str(&index_json)
        .unwrap_or_default();
    
    Ok(index)
}

// Save installations index
pub fn save_installations_index(index: &InstallationsIndex) -> Result<(), io::Error> {
    let installations_dir = get_installations_dir();
    
    // Create directory if it doesn't exist
    if !installations_dir.exists() {
        fs::create_dir_all(&installations_dir)?;
    }
    
    let index_path = installations_dir.join("index.json");
    let index_json = serde_json::to_string_pretty(index)?;
    
    fs::write(index_path, index_json)
}

// Register a new installation in the index
pub fn register_installation(installation: &Installation) -> Result<(), String> {
    let mut index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    // Add to index if not already present
    if !index.installations.contains(&installation.id) {
        index.installations.push(installation.id.clone());
    }
    
    // If this is the first installation, make it active
    if index.active_installation.is_none() {
        index.active_installation = Some(installation.id.clone());
    }
    
    index.last_active = Some(Utc::now());
    
    save_installations_index(&index)
        .map_err(|e| format!("Failed to save installations index: {}", e))
}

// Get the installations directory
pub fn get_installations_dir() -> PathBuf {
    let app_data = crate::get_app_data();
    app_data.join(".WC_OVHL/installations")
}
