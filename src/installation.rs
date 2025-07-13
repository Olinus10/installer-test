use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use log::{debug, error, info, warn};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;
use std::sync::Arc;
use std::sync::Mutex;

use crate::{CachedHttpClient, launcher};
use crate::preset::Preset;
use crate::Launcher;

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct InstallationsIndex {
    pub installations: Vec<String>,  // List of installation IDs
    pub active_installation: Option<String>, // Currently selected installation
    pub last_active: Option<DateTime<Utc>>,
}

pub fn get_installations_dir() -> PathBuf {
    let app_data = crate::get_app_data();
    app_data.join(".WC_OVHL/installations")
}

pub struct ProgressTracker {
    current: Arc<Mutex<i32>>,
    total: Arc<Mutex<i32>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            current: Arc::new(Mutex::new(0)),
            total: Arc::new(Mutex::new(0)),
        }
    }
    
    pub fn set_total(&self, total: i32) {
        *self.total.lock().unwrap() = total;
    }
    
    pub fn increment(&self) {
        *self.current.lock().unwrap() += 1;
    }
    
    pub fn get_progress(&self) -> (i32, i32) {
        (*self.current.lock().unwrap(), *self.total.lock().unwrap())
    }
}

// Function to load all installations
pub fn load_all_installations() -> Result<Vec<Installation>, String> {
    let index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    let mut installations = Vec::new();
    
    for id in &index.installations {
        match load_installation(id) {
            Ok(installation) => installations.push(installation),
            Err(e) => debug!("Failed to load installation {}: {}", id, e),
        }
    }
    
    // Sort by last used date (newest first)
    installations.sort_by(|a, b| b.last_used.cmp(&a.last_used));
    
    Ok(installations)
}

pub fn get_active_installation() -> Result<Installation, String> {
    let index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    if let Some(active_id) = index.active_installation {
        load_installation(&active_id)
    } else {
        Err("No active installation found".into())
    }
}

// Structure for managing an installation
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Installation {
    // Core identity properties
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,

    pub base_preset_id: Option<String>,
    pub base_preset_version: Option<String>,
    pub custom_features: Vec<String>,
    pub removed_features: Vec<String>,
    
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
    
    // Installation status tracking
    pub installed: bool,
    pub modified: bool,
    pub update_available: bool,
    pub preset_update_available: bool,  // Keep only this one
    
    
    // Launcher and versioning information
    pub launcher_type: String,    // "vanilla", "multimc", etc.
    pub universal_version: String, // Which version of the universal modpack this uses
    
    // Last launch info for statistics
    pub last_launch: Option<DateTime<Utc>>,
    pub total_launches: u32,
}

impl Installation {
    // Enhanced method to properly initialize from preset with full state tracking
    pub fn new_from_preset(
        name: String,
        preset: &Preset,
        minecraft_version: String,
        loader_type: String,
        loader_version: String,
        launcher_type: String,
        universal_version: String,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        
        info!("Creating new installation '{}' with preset '{}' (ID: {})", name, preset.name, id);
        
        let installations_dir = get_installations_dir();
        let installation_path = installations_dir.join(&id);
        
        let memory_allocation = preset.recommended_memory.unwrap_or(3072);
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
            installed: false,
            modified: false,
            update_available: false,
            launcher_type,
            universal_version,
            last_launch: None,
            total_launches: 0,
            preset_update_available: false,
            base_preset_id: Some(preset.id.clone()),
            base_preset_version: preset.preset_version.clone(),
            custom_features: Vec::new(),
            removed_features: Vec::new(),
        }
    }

    // Enhanced custom installation method
    pub fn new_custom(
        name: String,
        minecraft_version: String,
        loader_type: String,
        loader_version: String,
        launcher_type: String,
        universal_version: String,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        
        info!("Creating new custom installation '{}' (ID: {})", name, id);
        
        let installations_dir = get_installations_dir();
        let installation_path = installations_dir.join(&id);
        
        Self {
            id,
            name,
            created_at: now,
            last_used: now,
            minecraft_version,
            loader_type,
            loader_version,
            installation_path,
            enabled_features: vec!["default".to_string()], // Start with only default
            memory_allocation: 3072,
            java_args: "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M".to_string(),
            installed: false,
            modified: false,
            update_available: false,
            launcher_type,
            universal_version,
            last_launch: None,
            total_launches: 0,
            preset_update_available: false,
            base_preset_id: None, // No preset for custom
            base_preset_version: None,
            custom_features: Vec::new(),
            removed_features: Vec::new(),
        }
    }

    // Method to initialize enabled features based on universal manifest
    pub async fn initialize_default_features(&mut self, http_client: &crate::CachedHttpClient) -> Result<(), String> {
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        let mut features = vec!["default".to_string()];
        
        // Add all default-enabled components
        for component in &universal_manifest.mods {
            if component.default_enabled && !features.contains(&component.id) {
                features.push(component.id.clone());
            }
        }
        
        for component in &universal_manifest.shaderpacks {
            if component.default_enabled && !features.contains(&component.id) {
                features.push(component.id.clone());
            }
        }
        
        for component in &universal_manifest.resourcepacks {
            if component.default_enabled && !features.contains(&component.id) {
                features.push(component.id.clone());
            }
        }
        
        for include in &universal_manifest.include {
            if include.default_enabled && !include.id.is_empty() && include.id != "default" 
               && !features.contains(&include.id) {
                features.push(include.id.clone());
            }
        }
        
        for remote in &universal_manifest.remote_include {
            if remote.default_enabled && remote.id != "default" 
               && !features.contains(&remote.id) {
                features.push(remote.id.clone());
            }
        }
        
        self.enabled_features = features;
        debug!("Initialized default features: {:?}", self.enabled_features);
        Ok(())
    }

    // Method to load and restore user's previous choices
    pub fn restore_user_choices(&mut self) -> Result<(), String> {
        // If installation exists and has saved state, use it
        if self.installed {
            debug!("Restoring choices for installed modpack: {}", self.name);
            debug!("  Preset: {:?}", self.base_preset_id);
            debug!("  Enabled features: {:?}", self.enabled_features);
            debug!("  Custom features: {:?}", self.custom_features);
            debug!("  Removed features: {:?}", self.removed_features);
            return Ok(());
        }
        
        // For new installations, initialize with defaults
        if self.enabled_features.is_empty() || self.enabled_features == vec!["default".to_string()] {
            debug!("New installation, will initialize with defaults");
        }
        
        Ok(())
    }

    // Method to apply preset while tracking changes
    pub fn apply_preset_with_tracking(&mut self, preset: &crate::preset::Preset) {
        debug!("Applying preset '{}' to installation '{}'", preset.name, self.name);
        
        // Store previous state for comparison
        let previous_features = self.enabled_features.clone();
        let previous_preset = self.base_preset_id.clone();
        
        // Apply the preset
        self.base_preset_id = Some(preset.id.clone());
        self.base_preset_version = preset.preset_version.clone();
        self.enabled_features = preset.enabled_features.clone();
        
        // Clear modification tracking since we're applying a fresh preset
        self.custom_features.clear();
        self.removed_features.clear();
        
        self.modified = true;
        
        // Apply performance settings if provided
        if let Some(memory) = preset.recommended_memory {
            self.memory_allocation = memory;
        }
        
        if let Some(java_args) = &preset.recommended_java_args {
            self.java_args = java_args.clone();
        }
        
        debug!("Applied preset - features changed from {:?} to {:?}", previous_features, self.enabled_features);
    }

    // Method to switch to custom configuration
    pub fn switch_to_custom_with_tracking(&mut self) {
        debug!("Switching installation '{}' to custom configuration", self.name);
        
        // If switching from a preset, track what was changed
        if let Some(preset_id) = &self.base_preset_id {
            debug!("Switching from preset '{}' to custom", preset_id);
        }
        
        // Clear preset tracking
        self.base_preset_id = None;
        self.base_preset_version = None;
        
        // Keep existing features but clear change tracking
        self.custom_features.clear();
        self.removed_features.clear();
        
        self.modified = true;
        
        debug!("Switched to custom - current features: {:?}", self.enabled_features);
    }

    // Method to track individual feature changes
    pub fn toggle_feature_with_tracking(&mut self, feature_id: &str, enable: bool, presets: &[crate::preset::Preset]) {
        debug!("Toggling feature '{}' to {} for installation '{}'", feature_id, enable, self.name);
        
        // Update the feature state
        if enable {
            if !self.enabled_features.contains(&feature_id.to_string()) {
                self.enabled_features.push(feature_id.to_string());
            }
        } else {
            self.enabled_features.retain(|id| id != feature_id);
        }
        
        // Track changes relative to base preset
        if let Some(base_preset_id) = &self.base_preset_id {
            if let Some(base_preset) = presets.iter().find(|p| p.id == *base_preset_id) {
                let was_in_preset = base_preset.enabled_features.iter().any(|id| id == feature_id);
                
                if was_in_preset && !enable {
                    // Feature was removed from preset
                    if !self.removed_features.contains(&feature_id.to_string()) {
                        self.removed_features.push(feature_id.to_string());
                    }
                    self.custom_features.retain(|id| id != feature_id);
                } else if !was_in_preset && enable {
                    // Feature was added to preset
                    if !self.custom_features.contains(&feature_id.to_string()) {
                        self.custom_features.push(feature_id.to_string());
                    }
                    self.removed_features.retain(|id| id != feature_id);
                } else {
                    // Feature matches preset, remove from custom/removed lists
                    self.custom_features.retain(|id| id != feature_id);
                    self.removed_features.retain(|id| id != feature_id);
                }
            }
        }
        
        self.modified = true;
        
        debug!("Feature toggle complete - enabled: {:?}, custom: {:?}, removed: {:?}", 
               self.enabled_features, self.custom_features, self.removed_features);
    }

    // Enhanced completion method that preserves user choices
    pub async fn complete_installation_with_choices(&mut self, http_client: &crate::CachedHttpClient) -> Result<(), String> {
        debug!("Completing installation for '{}' while preserving user choices", self.name);
        
        // Load latest manifest to get current version
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        // Preserve user's enabled features - this is critical
        let user_features = self.enabled_features.clone();
        let user_preset = self.base_preset_id.clone();
        let user_preset_version = self.base_preset_version.clone();
        let user_custom_features = self.custom_features.clone();
        let user_removed_features = self.removed_features.clone();
        
        // Update installation state
        self.installed = true;
        self.update_available = false;
        self.modified = false;
        self.universal_version = universal_manifest.modpack_version;
        self.last_used = chrono::Utc::now();
        
        // CRITICAL: Restore user's choices
        self.enabled_features = user_features;
        self.base_preset_id = user_preset;
        self.base_preset_version = user_preset_version;
        self.custom_features = user_custom_features;
        self.removed_features = user_removed_features;
        
        debug!("Preserved user choices - features: {:?}, preset: {:?}", 
               self.enabled_features, self.base_preset_id);
        
        self.save()
    }

    // Method to check if installation needs to restore state from manifest
    pub fn needs_state_restoration(&self) -> bool {
        !self.installed || self.enabled_features.is_empty() || self.enabled_features == vec!["default".to_string()]
    }
}

// Register installation function for installation.rs
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
    
    index.last_active = Some(chrono::Utc::now());
    
    save_installations_index(&index)
        .map_err(|e| format!("Failed to save installations index: {}", e))
}

// Additional index loading/saving helpers for installation.rs
pub fn load_installations_index() -> Result<InstallationsIndex, std::io::Error> {
    let index_path = get_installations_dir().join("index.json");
    
    if !index_path.exists() {
        return Ok(InstallationsIndex::default());
    }
    
    let index_json = std::fs::read_to_string(index_path)?;
    let index: InstallationsIndex = serde_json::from_str(&index_json)
        .unwrap_or_default();
    
    Ok(index)
}

pub fn save_installations_index(index: &InstallationsIndex) -> Result<(), std::io::Error> {
    let installations_dir = get_installations_dir();
    
    // Create directory if it doesn't exist
    if !installations_dir.exists() {
        std::fs::create_dir_all(&installations_dir)?;
    }
    
    let index_path = installations_dir.join("index.json");
    let index_json = serde_json::to_string_pretty(index)?;
    
    std::fs::write(index_path, index_json)
}

// Load an installation by ID
pub fn load_installation(id: &str) -> Result<Installation, String> {
    let installation_dir = get_installations_dir().join(id);
    let config_path = installation_dir.join("installation.json");
    
    if !config_path.exists() {
        return Err(format!("Installation {} not found", id));
    }
    
    let config_json = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read installation config: {}", e))?;
    
    let installation: Installation = serde_json::from_str(&config_json)
        .map_err(|e| format!("Failed to parse installation config: {}", e))?;
    
    Ok(installation)
}

// Delete an installation
pub fn delete_installation(id: &str) -> Result<(), String> {
    debug!("Starting deletion of installation: {}", id);
    
    // Load the installation to get its launcher type before deleting
    let installation = load_installation(id)
        .map_err(|e| format!("Failed to load installation for deletion: {}", e))?;
    
    // Delete the launcher profile first
    if let Err(e) = crate::delete_launcher_profile(&installation.id, &installation.launcher_type) {
        debug!("Warning: Failed to delete launcher profile: {}", e);
        // Continue with deletion even if launcher profile deletion fails
    }
    
    // Remove from index
    let mut index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    index.installations.retain(|i| i != id);
    
    // If this was the active installation, clear it
    if index.active_installation.as_ref().map_or(false, |active| active == id) {
        index.active_installation = None;
    }
    
    save_installations_index(&index)
        .map_err(|e| format!("Failed to save installations index: {}", e))?;
    
    // Delete installation directory
    let installation_dir = get_installations_dir().join(id);
    if installation_dir.exists() {
        std::fs::remove_dir_all(&installation_dir)
            .map_err(|e| format!("Failed to delete installation directory: {}", e))?;
    }
    
    debug!("Successfully deleted installation: {}", id);
    Ok(())
}
