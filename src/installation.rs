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
    pub installations: Vec<String>,
    pub active_installation: Option<String>,
    pub last_active: Option<DateTime<Utc>>,
}

pub fn get_installations_dir() -> PathBuf {
    let app_data = crate::get_app_data();
    app_data.join(".WC_OVHL/installations")
}

// FIXED: Enhanced Installation struct with proper tracking
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Installation {
    // Core identity properties
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,

    // CRITICAL FIX: Enhanced preset and feature tracking
    pub base_preset_id: Option<String>,
    pub base_preset_version: Option<String>,
    pub selected_preset_id: Option<String>, // Currently selected preset (may differ from base)
    
    // FIXED: Comprehensive feature tracking
    pub enabled_features: Vec<String>,
    pub user_enabled_optional_features: Vec<String>, // Track user's optional choices
    pub user_disabled_default_features: Vec<String>, // Track if user disabled any defaults
    
    // Minecraft configuration
    pub minecraft_version: String,
    pub loader_type: String,
    pub loader_version: String,
    
    // Path information
    pub installation_path: PathBuf,
    
    // Performance settings
    pub memory_allocation: i32,
    pub java_args: String,
    
    // Installation status tracking
    pub installed: bool,
    pub modified: bool,
    pub update_available: bool,
    pub preset_update_available: bool,
    
    // Launcher and versioning information
    pub launcher_type: String,
    pub universal_version: String,
    
    // Last launch info for statistics
    pub last_launch: Option<DateTime<Utc>>,
    pub total_launches: u32,
}

impl Installation {
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
            user_enabled_optional_features: preset.enabled_features.iter()
                .filter(|id| *id != "default")
                .cloned()
                .collect(),
            user_disabled_default_features: Vec::new(),
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
            selected_preset_id: Some(preset.id.clone()), // Track current selection
        }
    }

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
            enabled_features: vec!["default".to_string()],
            user_enabled_optional_features: Vec::new(),
            user_disabled_default_features: Vec::new(),
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
            base_preset_id: None,
            base_preset_version: None,
            selected_preset_id: None,
        }
    }

    // CRITICAL FIX: Proper initialization with universal defaults
    pub async fn initialize_with_universal_defaults(&mut self, http_client: &crate::CachedHttpClient) -> Result<(), String> {
        debug!("Initializing installation '{}' with universal defaults", self.name);
        
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        let mut default_features = vec!["default".to_string()];
        let mut all_default_features = Vec::new(); // Track what should be considered "default"
        
        debug!("Processing universal manifest components for defaults...");
        
        // FIXED: Process ALL default-enabled components
        for component in &universal_manifest.mods {
            if component.default_enabled && component.id != "default" {
                default_features.push(component.id.clone());
                all_default_features.push(component.id.clone());
                debug!("Added default mod: {} ({})", component.name, component.id);
            }
        }
        
        for component in &universal_manifest.shaderpacks {
            if component.default_enabled && component.id != "default" {
                default_features.push(component.id.clone());
                all_default_features.push(component.id.clone());
                debug!("Added default shaderpack: {} ({})", component.name, component.id);
            }
        }
        
        for component in &universal_manifest.resourcepacks {
            if component.default_enabled && component.id != "default" {
                default_features.push(component.id.clone());
                all_default_features.push(component.id.clone());
                debug!("Added default resourcepack: {} ({})", component.name, component.id);
            }
        }
        
        // FIXED: Process includes properly
        for include in &universal_manifest.include {
            if include.default_enabled && !include.id.is_empty() && include.id != "default" {
                default_features.push(include.id.clone());
                all_default_features.push(include.id.clone());
                debug!("Added default include: {} ({})", include.location, include.id);
            } else if !include.optional && !include.id.is_empty() && include.id != "default" {
                // Non-optional includes are also defaults
                default_features.push(include.id.clone());
                all_default_features.push(include.id.clone());
                debug!("Added required include: {} ({})", include.location, include.id);
            }
        }
        
        // FIXED: Process remote includes properly  
        for remote in &universal_manifest.remote_include {
            if remote.default_enabled && remote.id != "default" {
                default_features.push(remote.id.clone());
                all_default_features.push(remote.id.clone());
                debug!("Added default remote include: {} ({})", remote.id, remote.id);
            } else if !remote.optional && remote.id != "default" {
                // Non-optional remote includes are also defaults
                default_features.push(remote.id.clone());
                all_default_features.push(remote.id.clone());
                debug!("Added required remote include: {} ({})", remote.id, remote.id);
            }
        }
        
        // Remove duplicates
        default_features.sort();
        default_features.dedup();
        
        debug!("Final default features list: {:?}", default_features);
        debug!("Total default components: {}", default_features.len());
        
        // Set the features
        self.enabled_features = default_features;
        self.user_enabled_optional_features.clear(); // Start fresh for custom
        self.user_disabled_default_features.clear();
        
        // Mark as custom since we're not using a preset
        self.base_preset_id = None;
        self.base_preset_version = None;
        self.selected_preset_id = None;
        
        debug!("Initialized custom installation with {} default features", self.enabled_features.len());
        Ok(())
    }

    // FIXED: Proper preset application with tracking
    pub fn apply_preset_with_tracking(&mut self, preset: &crate::preset::Preset) {
        debug!("Applying preset '{}' to installation '{}'", preset.name, self.name);
        
        // Store previous state for debugging
        let previous_features = self.enabled_features.clone();
        
        // Apply the preset features completely
        self.enabled_features = preset.enabled_features.clone();
        
        // Track which optional features the user has from this preset
        self.user_enabled_optional_features = preset.enabled_features.iter()
            .filter(|id| *id != "default") // Don't track "default" as user choice
            .cloned()
            .collect();
        
        // Clear disabled defaults since we're applying a fresh preset
        self.user_disabled_default_features.clear();
        
        // Update preset tracking
        self.base_preset_id = Some(preset.id.clone());
        self.base_preset_version = preset.preset_version.clone();
        self.selected_preset_id = Some(preset.id.clone());
        
        // Apply performance settings if provided
        if let Some(memory) = preset.recommended_memory {
            self.memory_allocation = memory;
        }
        
        if let Some(java_args) = &preset.recommended_java_args {
            self.java_args = java_args.clone();
        }
        
        self.modified = true;
        
        debug!("Applied preset '{}' - features changed from {} to {} items", 
               preset.name, previous_features.len(), self.enabled_features.len());
        debug!("User optional features: {:?}", self.user_enabled_optional_features);
        debug!("Selected preset ID: {:?}", self.selected_preset_id);
    }

    // FIXED: Enhanced feature toggling with proper tracking
    pub async fn toggle_feature_with_tracking(&mut self, feature_id: &str, enable: bool, http_client: &crate::CachedHttpClient) -> Result<(), String> {
        debug!("Toggling feature '{}' to {} for installation '{}'", feature_id, enable, self.name);
        
        // Load universal manifest to check if this is a default feature
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        // Check if this feature is considered "default" in the universal manifest
        let is_default_feature = self.is_default_feature_in_universal(&universal_manifest, feature_id);
        
        // Update the feature state
        if enable {
            if !self.enabled_features.contains(&feature_id.to_string()) {
                self.enabled_features.push(feature_id.to_string());
            }
            
            // Track user choices appropriately
            if is_default_feature {
                // If it's a default feature being re-enabled, remove from disabled list
                self.user_disabled_default_features.retain(|id| id != feature_id);
            } else {
                // If it's an optional feature being enabled, add to enabled list
                if !self.user_enabled_optional_features.contains(&feature_id.to_string()) {
                    self.user_enabled_optional_features.push(feature_id.to_string());
                }
            }
        } else {
            self.enabled_features.retain(|id| id != feature_id);
            
            // Track user choices appropriately
            if is_default_feature {
                // If it's a default feature being disabled, track that
                if !self.user_disabled_default_features.contains(&feature_id.to_string()) {
                    self.user_disabled_default_features.push(feature_id.to_string());
                }
            } else {
                // If it's an optional feature being disabled, remove from enabled list
                self.user_enabled_optional_features.retain(|id| id != feature_id);
            }
        }
        
        // If user makes changes, clear the selected preset to indicate custom state
        if !self.user_enabled_optional_features.is_empty() || !self.user_disabled_default_features.is_empty() {
            self.selected_preset_id = None; // User has customized beyond the preset
        }
        
        self.modified = true;
        
        debug!("Feature toggle complete - enabled: {:?}", self.enabled_features);
        debug!("User optional features: {:?}", self.user_enabled_optional_features);
        debug!("User disabled defaults: {:?}", self.user_disabled_default_features);
        
        Ok(())
    }
    
    // Helper to check if a feature is default in the universal manifest
    fn is_default_feature_in_universal(&self, universal_manifest: &crate::universal::UniversalManifest, feature_id: &str) -> bool {
        if feature_id == "default" {
            return true;
        }
        
        // Check mods
        for component in &universal_manifest.mods {
            if component.id == feature_id && (component.default_enabled || !component.optional) {
                return true;
            }
        }
        
        // Check shaderpacks
        for component in &universal_manifest.shaderpacks {
            if component.id == feature_id && (component.default_enabled || !component.optional) {
                return true;
            }
        }
        
        // Check resourcepacks
        for component in &universal_manifest.resourcepacks {
            if component.id == feature_id && (component.default_enabled || !component.optional) {
                return true;
            }
        }
        
        // Check includes
        for include in &universal_manifest.include {
            if include.id == feature_id && (include.default_enabled || !include.optional) {
                return true;
            }
        }
        
        // Check remote includes
        for remote in &universal_manifest.remote_include {
            if remote.id == feature_id && (remote.default_enabled || !remote.optional) {
                return true;
            }
        }
        
        false
    }

    // FIXED: Get the effective preset considering user customizations
    pub fn get_effective_preset_id(&self, presets: &[crate::preset::Preset]) -> Option<String> {
        // If user has made no customizations, return the selected preset
        if self.user_enabled_optional_features.is_empty() && self.user_disabled_default_features.is_empty() {
            return self.selected_preset_id.clone();
        }
        
        // If user has customizations, check if current config exactly matches any preset
        for preset in presets {
            if self.matches_preset_exactly(preset) {
                debug!("Installation exactly matches preset: {}", preset.id);
                return Some(preset.id.clone());
            }
        }
        
        // If no exact match and user has customizations, it's custom
        None
    }
    
    // Helper to check exact preset match
    fn matches_preset_exactly(&self, preset: &crate::preset::Preset) -> bool {
        let mut our_features = self.enabled_features.clone();
        let mut preset_features = preset.enabled_features.clone();
        
        our_features.sort();
        preset_features.sort();
        
        our_features == preset_features
    }

    // Enhanced completion method that preserves user choices
    pub async fn complete_installation_with_choices(&mut self, http_client: &crate::CachedHttpClient) -> Result<(), String> {
        debug!("Completing installation for '{}' while preserving user choices", self.name);
        
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        // CRITICAL: Preserve all user state
        let user_features = self.enabled_features.clone();
        let user_optional = self.user_enabled_optional_features.clone();
        let user_disabled = self.user_disabled_default_features.clone();
        let selected_preset = self.selected_preset_id.clone();
        let base_preset = self.base_preset_id.clone();
        let base_preset_version = self.base_preset_version.clone();
        
        // Update installation state
        self.installed = true;
        self.update_available = false;
        self.modified = false;
        self.universal_version = universal_manifest.modpack_version;
        self.last_used = chrono::Utc::now();
        
        // CRITICAL: Restore user's choices
        self.enabled_features = user_features;
        self.user_enabled_optional_features = user_optional;
        self.user_disabled_default_features = user_disabled;
        self.selected_preset_id = selected_preset;
        self.base_preset_id = base_preset;
        self.base_preset_version = base_preset_version;
        
        debug!("Preserved user choices - features: {:?}", self.enabled_features);
        debug!("Optional features: {:?}", self.user_enabled_optional_features);
        debug!("Disabled defaults: {:?}", self.user_disabled_default_features);
        debug!("Selected preset: {:?}", self.selected_preset_id);
        
        self.save()
    }

    // FIXED: Enhanced install/update with proper progress tracking
    pub async fn install_or_update_with_progress<F: FnMut() + Clone>(
        &self, 
        http_client: &crate::CachedHttpClient,
        progress_callback: F
    ) -> Result<(), String> {
        debug!("Starting install/update for: {}", self.name);
        debug!("Enabled features: {:?}", self.enabled_features);
        
        // Load universal manifest
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        // Convert to regular manifest with user's enabled features
        let mut manifest = crate::universal::universal_to_manifest(
            &universal_manifest, 
            self.enabled_features.clone()
        );
        
        // Override important properties
        manifest.uuid = self.id.clone();
        manifest.name = self.name.clone();
        
        // Create launcher
        let launcher = match self.launcher_type.as_str() {
            "vanilla" => {
                let app_data = crate::get_app_data();
                Ok(crate::Launcher::Vanilla(app_data))
            },
            launcher_type if launcher_type.starts_with("multimc") => {
                let launcher_name = if launcher_type == "multimc-MultiMC" {
                    "MultiMC"
                } else if launcher_type == "multimc-PrismLauncher" {
                    "PrismLauncher"
                } else {
                    return Err(format!("Unsupported MultiMC launcher type: {}", launcher_type));
                };
                crate::get_multimc_folder(launcher_name).map(crate::Launcher::MultiMC)
            },
            custom if custom.starts_with("custom-") => {
                let path = custom.trim_start_matches("custom-");
                Ok(crate::Launcher::MultiMC(std::path::PathBuf::from(path)))
            },
            _ => Err(format!("Unsupported launcher type: {}", self.launcher_type)),
        }?;
        
        let installer_profile = crate::InstallerProfile {
            manifest,
            http_client: http_client.clone(),
            installed: self.installed,
            update_available: self.update_available,
            modpack_source: "Wynncraft-Overhaul/majestic-overhaul/".to_string(),
            modpack_branch: "master".to_string(),
            enabled_features: self.enabled_features.clone(), // CRITICAL: Use user's choices
            launcher: Some(launcher),
            local_manifest: None,
            changelog: None,
        };
        
        // Run installation with user's exact choices
        debug!("Running installation with user features: {:?}", self.enabled_features);
        
        if !self.installed {
            crate::install(&installer_profile, progress_callback).await?;
        } else {
            crate::update(&installer_profile, progress_callback).await?;
        }
        
        Ok(())
    }

    // Other existing methods remain the same...
    pub fn save(&self) -> Result<(), String> {
        let installation_dir = get_installations_dir().join(&self.id);
        
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

    pub fn mark_as_fresh(&mut self) {
        self.installed = false;
        self.modified = false;
        self.update_available = false;
        self.preset_update_available = false;
    }

    pub fn record_launch(&mut self) -> Result<(), String> {
        self.last_launch = Some(chrono::Utc::now());
        self.total_launches += 1;
        self.last_used = chrono::Utc::now();
        self.save()
    }

    pub async fn check_for_updates(&mut self, http_client: &CachedHttpClient, presets: &[Preset]) -> Result<bool, String> {
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        let modpack_update = match crate::compare_versions(&universal_manifest.modpack_version, &self.universal_version) {
            std::cmp::Ordering::Greater => {
                debug!("Modpack update available: {} -> {}", self.universal_version, universal_manifest.modpack_version);
                true
            },
            _ => false
        };
        
        let preset_update = if let Some(base_preset_id) = &self.base_preset_id {
            if let Some(current_preset) = presets.iter().find(|p| p.id == *base_preset_id) {
                if let (Some(current_version), Some(base_version)) = 
                    (&current_preset.preset_version, &self.base_preset_version) {
                    match crate::compare_versions(current_version, base_version) {
                        std::cmp::Ordering::Greater => {
                            debug!("Preset update available: {} -> {}", base_version, current_version);
                            true
                        },
                        _ => false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        
        self.update_available = modpack_update || preset_update;
        self.preset_update_available = preset_update;
        
        self.save()?;
        Ok(self.update_available)
    }
}

// Keep all the existing helper functions...
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
    
    installations.sort_by(|a, b| b.last_used.cmp(&a.last_used));
    Ok(installations)
}

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

pub fn register_installation(installation: &Installation) -> Result<(), String> {
    let mut index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    if !index.installations.contains(&installation.id) {
        index.installations.push(installation.id.clone());
    }
    
    if index.active_installation.is_none() {
        index.active_installation = Some(installation.id.clone());
    }
    
    index.last_active = Some(chrono::Utc::now());
    
    save_installations_index(&index)
        .map_err(|e| format!("Failed to save installations index: {}", e))
}

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
    
    if !installations_dir.exists() {
        std::fs::create_dir_all(&installations_dir)?;
    }
    
    let index_path = installations_dir.join("index.json");
    let index_json = serde_json::to_string_pretty(index)?;
    
    std::fs::write(index_path, index_json)
}

pub fn get_active_installation() -> Result<Installation, String> {
    let index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    if let Some(active_id) = index.active_installation {
        load_installation(&active_id)
    } else {
        // If no active installation, return the most recently used one
        let installations = load_all_installations()?;
        installations.into_iter().next()
            .ok_or_else(|| "No installations found".to_string())
    }
}

pub fn delete_installation(id: &str) -> Result<(), String> {
    debug!("Starting deletion of installation: {}", id);
    
    let installation = load_installation(id)
        .map_err(|e| format!("Failed to load installation for deletion: {}", e))?;
    
    if let Err(e) = crate::delete_launcher_profile(&installation.id, &installation.launcher_type) {
        debug!("Warning: Failed to delete launcher profile: {}", e);
    }
    
    let mut index = load_installations_index()
        .map_err(|e| format!("Failed to load installations index: {}", e))?;
    
    index.installations.retain(|i| i != id);
    
    if index.active_installation.as_ref().map_or(false, |active| active == id) {
        index.active_installation = None;
    }
    
    save_installations_index(&index)
        .map_err(|e| format!("Failed to save installations index: {}", e))?;
    
    let installation_dir = get_installations_dir().join(id);
    if installation_dir.exists() {
        std::fs::remove_dir_all(&installation_dir)
            .map_err(|e| format!("Failed to delete installation directory: {}", e))?;
    }
    
    debug!("Successfully deleted installation: {}", id);
    Ok(())
}
