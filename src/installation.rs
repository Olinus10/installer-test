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

// Helper function to compare semantic versions
fn compare_versions(version1: &str, version2: &str) -> std::cmp::Ordering {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect()
    };
    
    let v1_parts = parse_version(version1);
    let v2_parts = parse_version(version2);
    
    let max_len = std::cmp::max(v1_parts.len(), v2_parts.len());
    
    for i in 0..max_len {
        let v1_part = v1_parts.get(i).unwrap_or(&0);
        let v2_part = v2_parts.get(i).unwrap_or(&0);
        
        match v1_part.cmp(v2_part) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    
    std::cmp::Ordering::Equal
}

// Check if a version is newer than another
fn is_version_newer(current: &str, new: &str) -> bool {
    compare_versions(new, current) == std::cmp::Ordering::Greater
}

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
    pub preset_update_available: bool,
    
    // Launcher and versioning information
    pub launcher_type: String,    // "vanilla", "multimc", etc.
    pub universal_version: String, // Which version of the universal modpack this uses
    
    // Last launch info for statistics
    pub last_launch: Option<DateTime<Utc>>,
    pub total_launches: u32,
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
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        
        info!("Creating new installation '{}' with ID: {}", name, id);
        
        // Generate installation path based on ID
        let installations_dir = get_installations_dir();
        let installation_path = installations_dir.join(&id);
        
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

    // Custom installation without using a preset
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
        
        info!("Creating new custom installation '{}' with ID: {}", name, id);
        
        // Generate installation path based on ID
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
            enabled_features: vec!["default".to_string()],
            memory_allocation: 3072, // 3GB default
            java_args: "-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M".to_string(),
            installed: false,
            modified: false,
            update_available: false,
            launcher_type,
            universal_version,
            last_launch: None,
            total_launches: 0,
            preset_update_available: false,
            base_preset_id: None,
            base_preset_version: None,
            custom_features: Vec::new(),
            removed_features: Vec::new(),
        }
    }

    // NEW: Check for modpack updates by comparing universal manifest versions
    pub async fn check_modpack_updates(&mut self, http_client: &CachedHttpClient) -> Result<bool, String> {
        debug!("Checking for modpack updates for installation: {}", self.id);
        
        // Load the current universal manifest
        let universal_manifest = match crate::universal::load_universal_manifest(http_client, None).await {
            Ok(manifest) => manifest,
            Err(e) => {
                warn!("Failed to load universal manifest for update check: {}", e.message);
                return Ok(false); // Don't fail completely, just no update available
            }
        };
        
        let remote_version = &universal_manifest.modpack_version;
        let local_version = &self.universal_version;
        
        debug!("Comparing versions: local={}, remote={}", local_version, remote_version);
        
        // Check if remote version is newer
        let modpack_update_available = is_version_newer(local_version, remote_version);
        
        if modpack_update_available {
            info!("Modpack update available: {} -> {}", local_version, remote_version);
        }
        
        self.update_available = modpack_update_available;
        
        Ok(modpack_update_available)
    }

    // NEW: Check for preset updates by comparing preset versions
    pub async fn check_preset_updates(&mut self, presets: &[Preset]) -> Result<bool, String> {
        if let Some(base_preset_id) = &self.base_preset_id {
            debug!("Checking for preset updates for preset: {}", base_preset_id);
            
            if let Some(current_preset) = presets.iter().find(|p| p.id == *base_preset_id) {
                // Check if preset version has changed
                if let (Some(current_version), Some(base_version)) = 
                    (&current_preset.preset_version, &self.base_preset_version) {
                    
                    debug!("Comparing preset versions: local={}, remote={}", base_version, current_version);
                    
                    let preset_update_available = is_version_newer(base_version, current_version);
                    
                    if preset_update_available {
                        info!("Preset update available: {} -> {}", base_version, current_version);
                    }
                    
                    self.preset_update_available = preset_update_available;
                    
                    return Ok(preset_update_available);
                }
            } else {
                warn!("Base preset {} not found in current presets", base_preset_id);
            }
        }
        
        self.preset_update_available = false;
        Ok(false)
    }

    // NEW: Combined update check that handles both modpack and preset updates
    pub async fn check_for_updates(&mut self, http_client: &CachedHttpClient, presets: &[Preset]) -> Result<bool, String> {
        debug!("Performing comprehensive update check for installation: {}", self.id);
        
        // Check both modpack and preset updates
        let modpack_update = self.check_modpack_updates(http_client).await?;
        let preset_update = self.check_preset_updates(presets).await?;
        
        // If either has an update available, we show the update button
        let any_update_available = modpack_update || preset_update;
        
        // Store the combined result
        self.update_available = any_update_available;
        
        // Save the updated state
        self.save()?;
        
        debug!("Update check complete: modpack={}, preset={}, overall={}", 
               modpack_update, preset_update, any_update_available);
        
        Ok(any_update_available)
    }
    
    pub fn apply_preset_update(&mut self, preset: &Preset) {
        // Start with the preset's features
        let mut new_features = preset.enabled_features.clone();
        
        // Add custom features the user added
        for custom in &self.custom_features {
            if !new_features.contains(custom) {
                new_features.push(custom.clone());
            }
        }
        
        // Remove features the user removed
        for removed in &self.removed_features {
            new_features.retain(|f| f != removed);
        }
        
        self.enabled_features = new_features;
        self.base_preset_version = preset.preset_version.clone();
    }

    pub fn mark_installed(&mut self) -> Result<(), String> {
        self.installed = true;
        self.update_available = false;
        self.preset_update_available = false; // Clear both update flags
        self.modified = false;
        self.last_used = chrono::Utc::now();
        self.save()
    }

    pub fn save(&self) -> Result<(), String> {
        let installation_dir = get_installations_dir().join(&self.id);
        
        // Create directory if it doesn't exist
        if !installation_dir.exists() {
            std::fs::create_dir_all(&installation_dir)
                .map_err(|e| format!("Failed to create installation directory: {}", e))?;
        }
        
        let config_path = installation_dir.join("installation.json");
        let config_json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize installation: {}", e))?;
        
        std::fs::write(config_path, config_json)
            .map_err(|e| format!("Failed to write installation config: {}", e))
    }
    
    pub async fn install_or_update(&self, http_client: &crate::CachedHttpClient) -> Result<(), String> {
        self.install_or_update_with_progress(http_client, || {}).await
    }
    
    pub async fn install_or_update_with_progress<F: FnMut() + Clone>(
        &self, 
        http_client: &crate::CachedHttpClient,
        progress_callback: F
    ) -> Result<(), String> {
        // Get the universal manifest
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e.message))?;
        
        // Convert universal manifest to regular manifest with our enabled features
        let mut manifest = crate::universal::universal_to_manifest(
            &universal_manifest, 
            self.enabled_features.clone()
        );
        
        // IMPORTANT: Override the UUID with this installation's ID
        manifest.uuid = self.id.clone();
        manifest.name = self.name.clone();
        
        // Create launcher
        let launcher = match self.launcher_type.as_str() {
            "vanilla" => {
                let app_data = crate::get_app_data();
                Ok(crate::Launcher::Vanilla(app_data))
            },
            "multimc" => crate::get_multimc_folder("MultiMC").map(crate::Launcher::MultiMC),
            "prismlauncher" => crate::get_multimc_folder("PrismLauncher").map(crate::Launcher::MultiMC),
            _ => Err(format!("Unsupported launcher type: {}", self.launcher_type)),
        }?;
        
        let installer_profile = crate::InstallerProfile {
            manifest,
            http_client: http_client.clone(),
            installed: self.installed,
            update_available: self.update_available,
            modpack_source: "Wynncraft-Overhaul/majestic-overhaul/".to_string(),
            modpack_branch: "master".to_string(),
            enabled_features: self.enabled_features.clone(),
            launcher: Some(launcher),
            local_manifest: None,
            changelog: None,
        };
        
        // Install or update based on current state
        if !self.installed {
            crate::install(&installer_profile, progress_callback).await?;
        } else {
            crate::update(&installer_profile, progress_callback).await?;
        }
        
        Ok(())
    }
    
    // Update the play method to increment launch count
    pub fn record_launch(&mut self) -> Result<(), String> {
        self.last_launch = Some(chrono::Utc::now());
        self.total_launches += 1;
        self.last_used = chrono::Utc::now();
        
        // Save the updated installation data
        self.save()
    }
    
    // Update the installation after successful install/update
    pub async fn complete_installation(&mut self, http_client: &CachedHttpClient) -> Result<(), String> {
        // Load latest manifest to get current version
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e.message))?;
        
        // Update installation state - IMPORTANT: Update version numbers here
        self.installed = true;
        self.update_available = false;
        self.preset_update_available = false;
        self.modified = false;
        self.universal_version = universal_manifest.modpack_version; // Update to new version
        self.last_used = chrono::Utc::now();
        
        // If we have a preset, update its version too
        if let Some(preset_id) = &self.base_preset_id {
            // We would need to get the current preset version here
            // This might require loading presets, but for now we'll handle it in the calling code
            debug!("Installation completed, preset version should be updated by calling code");
        }
        
        self.save()
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
