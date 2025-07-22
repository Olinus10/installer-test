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

    // Preset tracking - existing fields
    pub base_preset_id: Option<String>,
    pub base_preset_version: Option<String>,
    pub custom_features: Vec<String>,
    pub removed_features: Vec<String>,
    
    // NEW: Enhanced selection tracking
    pub selected_preset_id: Option<String>,        // The preset the user selected (None = custom)
    pub pre_install_features: Vec<String>,         // Features selected before first install
    pub installed_features: Vec<String>,           // Features that were actually installed
    pub pending_features: Vec<String>,             // Features selected but not yet installed
    pub is_custom_configuration: bool,             // True if user is using custom (no preset)
    
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
            // NEW: Initialize selection tracking
            selected_preset_id: Some(preset.id.clone()),
            pre_install_features: preset.enabled_features.clone(),
            installed_features: Vec::new(),
            pending_features: preset.enabled_features.clone(),
            is_custom_configuration: false,
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
            // NEW: Initialize selection tracking for custom
            selected_preset_id: None,
            pre_install_features: vec!["default".to_string()],
            installed_features: Vec::new(),
            pending_features: vec!["default".to_string()],
            is_custom_configuration: true,
        }
    }

        /// Get the backups directory for this installation
    pub fn get_backups_dir(&self) -> PathBuf {
        self.installation_path.join("backups")
    }
    
    /// List all available backups for this installation
    pub fn list_available_backups(&self) -> Result<Vec<crate::backup::BackupMetadata>, String> {
        let backups_dir = self.get_backups_dir();
        
        if !backups_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut backups = Vec::new();
        
        match std::fs::read_dir(&backups_dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_dir() {
                            let metadata_path = path.join("metadata.json");
                            if metadata_path.exists() {
                                match std::fs::read_to_string(&metadata_path) {
                                    Ok(content) => {
                                        match serde_json::from_str::<crate::backup::BackupMetadata>(&content) {
                                            Ok(metadata) => backups.push(metadata),
                                            Err(e) => debug!("Failed to parse backup metadata: {}", e),
                                        }
                                    },
                                    Err(e) => debug!("Failed to read backup metadata: {}", e),
                                }
                            }
                        }
                    }
                }
            },
            Err(e) => return Err(format!("Failed to read backups directory: {}", e)),
        }
        
        // Sort by creation date, newest first
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(backups)
    }
    
    /// Get size estimate for a backup with given configuration
    pub fn get_backup_size_estimate(&self, config: &crate::backup::BackupConfig) -> Result<u64, String> {
        let mut total_size = 0u64;
        
        let folders_to_check = self.get_backup_folders(config);
        
        for folder_path in folders_to_check {
            if folder_path.exists() {
                match crate::backup::calculate_directory_size(&folder_path) {
                    Ok(size) => total_size += size,
                    Err(e) => debug!("Failed to calculate size for {:?}: {}", folder_path, e),
                }
            }
        }
        
        // Apply compression estimate if enabled
        if config.compress_backups {
            // Estimate 35% compression ratio for typical modpack files
            total_size = (total_size as f64 * 0.65) as u64;
        }
        
        Ok(total_size)
    }
    
    /// Get list of folders to backup based on configuration
    fn get_backup_folders(&self, config: &crate::backup::BackupConfig) -> Vec<PathBuf> {
        let mut folders = Vec::new();
        
        if config.include_mods {
            folders.push(self.installation_path.join("mods"));
        }
        if config.include_config {
            folders.push(self.installation_path.join("config"));
        }
        if config.include_wynntils {
            folders.push(self.installation_path.join("wynntils"));
        }
        if config.include_resourcepacks {
            folders.push(self.installation_path.join("resourcepacks"));
        }
        if config.include_shaderpacks {
            folders.push(self.installation_path.join("shaderpacks"));
        }
        if config.include_saves {
            folders.push(self.installation_path.join("saves"));
        }
        if config.include_screenshots {
            folders.push(self.installation_path.join("screenshots"));
        }
        if config.include_logs {
            folders.push(self.installation_path.join("logs"));
        }
        
        folders
    }
    
    /// Create a backup of this installation
    pub async fn create_backup<F>(
        &self,
        backup_type: crate::backup::BackupType,
        config: &crate::backup::BackupConfig,
        description: String,
        progress_callback: Option<F>,
    ) -> Result<crate::backup::BackupMetadata, String>
    where
        F: FnMut(crate::backup::BackupProgress) + Send + 'static,
    {
        use chrono::Utc;
        use uuid::Uuid;
        
        let backup_id = Uuid::new_v4().to_string();
        let backups_dir = self.get_backups_dir();
        let backup_dir = backups_dir.join(&backup_id);
        
        // Create backup directory
        std::fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;
        
        debug!("Creating backup {} for installation {}", backup_id, self.name);
        
        // Count total files for progress tracking
        let folders_to_backup = self.get_backup_folders(config);
        let mut total_files = 0;
        let mut total_bytes = 0;
        
        for folder in &folders_to_backup {
            if folder.exists() {
                total_files += crate::backup::count_files_recursive(folder)
                    .unwrap_or(0);
                total_bytes += crate::backup::calculate_directory_size(folder)
                    .unwrap_or(0);
            }
        }
        
        debug!("Backup will include {} files ({} bytes)", total_files, total_bytes);
        
        let mut files_processed = 0;
        let mut bytes_processed = 0;
        
        // Copy or compress files
        if config.compress_backups {
            // Create ZIP archive
            let archive_path = backup_dir.join("backup.zip");
            let temp_dir = backup_dir.join("temp");
            std::fs::create_dir_all(&temp_dir)
                .map_err(|e| format!("Failed to create temp directory: {}", e))?;
            
            // Copy files to temp directory first
            for folder in &folders_to_backup {
                if folder.exists() {
                    let folder_name = folder.file_name()
                        .ok_or("Invalid folder name")?
                        .to_string_lossy();
                    let dest_folder = temp_dir.join(&*folder_name);
                    
                    self.copy_directory_with_progress(
                        folder,
                        &dest_folder,
                        &mut files_processed,
                        total_files,
                        &mut bytes_processed,
                        &progress_callback,
                    )?;
                }
            }
            
            // Create ZIP archive
            let final_size = crate::backup::create_zip_archive(
                &temp_dir,
                &archive_path,
                progress_callback.as_ref(),
            ).map_err(|e| format!("Failed to create ZIP archive: {}", e))?;
            
            // Clean up temp directory
            std::fs::remove_dir_all(&temp_dir)
                .map_err(|e| format!("Failed to clean up temp directory: {}", e))?;
            
            bytes_processed = final_size;
        } else {
            // Copy files directly
            for folder in &folders_to_backup {
                if folder.exists() {
                    let folder_name = folder.file_name()
                        .ok_or("Invalid folder name")?
                        .to_string_lossy();
                    let dest_folder = backup_dir.join(&*folder_name);
                    
                    self.copy_directory_with_progress(
                        folder,
                        &dest_folder,
                        &mut files_processed,
                        total_files,
                        &mut bytes_processed,
                        &progress_callback,
                    )?;
                }
            }
        }
        
        // Create metadata
        let metadata = crate::backup::BackupMetadata {
            id: backup_id.clone(),
            description,
            backup_type,
            created_at: Utc::now(),
            modpack_version: self.universal_version.clone(),
            enabled_features: self.enabled_features.clone(),
            file_count: files_processed,
            size_bytes: bytes_processed,
            config: config.clone(),
        };
        
        // Save metadata
        let metadata_path = backup_dir.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        std::fs::write(&metadata_path, metadata_json)
            .map_err(|e| format!("Failed to write metadata: {}", e))?;
        
        // Clean up old backups if needed
        self.cleanup_old_backups(config.max_backups)?;
        
        info!("Successfully created backup {} for installation {}", backup_id, self.name);
        Ok(metadata)
    }
    
    /// Helper method to copy directory with progress tracking
    fn copy_directory_with_progress<F>(
        &self,
        source: &PathBuf,
        dest: &PathBuf,
        files_processed: &mut usize,
        total_files: usize,
        bytes_processed: &mut u64,
        progress_callback: &Option<F>,
    ) -> Result<(), String>
    where
        F: Fn(crate::backup::BackupProgress),
    {
        if source.is_file() {
            // Copy single file
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }
            
            std::fs::copy(source, dest)
                .map_err(|e| format!("Failed to copy file: {}", e))?;
                
            let file_size = std::fs::metadata(dest)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .len();
                
            *files_processed += 1;
            *bytes_processed += file_size;
            
            if let Some(callback) = progress_callback {
                callback(crate::backup::BackupProgress {
                    current_file: dest.to_string_lossy().to_string(),
                    files_processed: *files_processed,
                    total_files,
                    bytes_processed: *bytes_processed,
                    total_bytes: 0, // Will be calculated separately
                });
            }
            
            return Ok(());
        }
        
        if !source.is_dir() {
            return Ok(());
        }
        
        std::fs::create_dir_all(dest)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        
        let entries = std::fs::read_dir(source)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let source_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            
            if source_path.is_dir() {
                self.copy_directory_with_progress(
                    &source_path,
                    &dest_path,
                    files_processed,
                    total_files,
                    bytes_processed,
                    progress_callback,
                )?;
            } else {
                self.copy_directory_with_progress(
                    &source_path,
                    &dest_path,
                    files_processed,
                    total_files,
                    bytes_processed,
                    progress_callback,
                )?;
            }
        }
        
        Ok(())
    }
    
    /// Restore installation from a backup
    pub async fn restore_from_backup(&mut self, backup_id: &str) -> Result<(), String> {
        let backup_dir = self.get_backups_dir().join(backup_id);
        
        if !backup_dir.exists() {
            return Err(format!("Backup {} not found", backup_id));
        }
        
        // Load backup metadata
        let metadata_path = backup_dir.join("metadata.json");
        let metadata_content = std::fs::read_to_string(&metadata_path)
            .map_err(|e| format!("Failed to read backup metadata: {}", e))?;
        let metadata: crate::backup::BackupMetadata = serde_json::from_str(&metadata_content)
            .map_err(|e| format!("Failed to parse backup metadata: {}", e))?;
        
        debug!("Restoring from backup {} ({})", backup_id, metadata.description);
        
        // Create safety backup before restore
        let safety_config = crate::backup::BackupConfig::default();
        let safety_description = format!("Safety backup before restore from {}", backup_id);
        self.create_backup(
            crate::backup::BackupType::PreUpdate,
            &safety_config,
            safety_description,
            None::<fn(crate::backup::BackupProgress)>,
        ).await?;
        
        // Clear existing installation folders that will be restored
        let folders_to_clear = self.get_backup_folders(&metadata.config);
        for folder in &folders_to_clear {
            if folder.exists() {
                std::fs::remove_dir_all(folder)
                    .map_err(|e| format!("Failed to remove existing folder: {}", e))?;
            }
        }
        
        // Restore from backup
        if metadata.config.compress_backups {
            // Extract from ZIP
            let archive_path = backup_dir.join("backup.zip");
            crate::backup::extract_zip_archive(&archive_path, &self.installation_path)
                .map_err(|e| format!("Failed to extract backup: {}", e))?;
        } else {
            // Copy files directly
            for folder in &folders_to_clear {
                let folder_name = folder.file_name()
                    .ok_or("Invalid folder name")?
                    .to_string_lossy();
                let source_folder = backup_dir.join(&*folder_name);
                
                if source_folder.exists() {
                    self.copy_directory_with_progress(
                        &source_folder,
                        folder,
                        &mut 0,
                        0,
                        &mut 0,
                        &None::<fn(crate::backup::BackupProgress)>,
                    )?;
                }
            }
        }
        
        // Update installation state
        self.enabled_features = metadata.enabled_features.clone();
        self.universal_version = metadata.modpack_version.clone();
        self.last_used = chrono::Utc::now();
        
        // Save the updated installation
        self.save()?;
        
        info!("Successfully restored installation from backup {}", backup_id);
        Ok(())
    }
    
    /// Delete a backup
    pub async fn delete_backup(&self, backup_id: &str) -> Result<(), String> {
        let backup_dir = self.get_backups_dir().join(backup_id);
        
        if !backup_dir.exists() {
            return Err(format!("Backup {} not found", backup_id));
        }
        
        std::fs::remove_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to delete backup: {}", e))?;
        
        debug!("Deleted backup {}", backup_id);
        Ok(())
    }
    
    /// Clean up old backups beyond the maximum limit
    fn cleanup_old_backups(&self, max_backups: usize) -> Result<(), String> {
        let mut backups = self.list_available_backups()?;
        
        if backups.len() <= max_backups {
            return Ok(());
        }
        
        // Sort by creation date, oldest first
        backups.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        
        // Remove oldest backups
        let to_remove = backups.len() - max_backups;
        for backup in backups.iter().take(to_remove) {
            let backup_dir = self.get_backups_dir().join(&backup.id);
            if backup_dir.exists() {
                std::fs::remove_dir_all(&backup_dir)
                    .map_err(|e| format!("Failed to cleanup old backup: {}", e))?;
                debug!("Cleaned up old backup: {}", backup.id);
            }
        }
        
        Ok(())
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

    pub fn mark_as_fresh(&mut self) {
        self.installed = false;
        self.modified = false;
        self.update_available = false;
        self.preset_update_available = false;
    }

    // NEW: Method to save user's pre-installation selections
    pub fn save_pre_install_selections(&mut self, preset_id: Option<String>, features: Vec<String>) {
        debug!("Saving pre-install selections - preset: {:?}, features: {:?}", preset_id, features);
        
        self.selected_preset_id = preset_id.clone();
        self.pre_install_features = features.clone();
        self.pending_features = features;
        self.is_custom_configuration = preset_id.is_none();
        
        if let Some(preset_id) = preset_id {
            self.base_preset_id = Some(preset_id);
        } else {
            self.base_preset_id = None;
        }
        
        self.save().unwrap_or_else(|e| {
            error!("Failed to save pre-install selections: {}", e);
        });
    }
    
    // NEW: Method to commit installation (called after successful install)
    pub fn commit_installation(&mut self) {
        debug!("Committing installation - pending features: {:?}", self.pending_features);
        
        self.installed_features = self.pending_features.clone();
        self.enabled_features = self.pending_features.clone();
        self.pending_features.clear();
        self.installed = true;
        self.modified = false;
        
        self.save().unwrap_or_else(|e| {
            error!("Failed to commit installation: {}", e);
        });
    }
    
    // NEW: Method to get the effective preset for UI display
    pub fn get_display_preset_id(&self) -> Option<String> {
        if self.is_custom_configuration {
            None // Custom configuration
        } else {
            self.selected_preset_id.clone()
        }
    }
    
    // NEW: Method to get the features that should be displayed as enabled in UI
    pub fn get_display_features(&self) -> Vec<String> {
        if self.installed {
            // Show what's actually installed
            self.installed_features.clone()
        } else if !self.pending_features.is_empty() {
            // Show what's pending to be installed
            self.pending_features.clone()
        } else if !self.pre_install_features.is_empty() {
            // Show pre-install selections
            self.pre_install_features.clone()
        } else {
            // Default to just "default"
            vec!["default".to_string()]
        }
    }

    pub async fn install_or_update_with_progress<F: FnMut() + Clone>(
        &self, 
        http_client: &crate::CachedHttpClient,
        progress_callback: F
    ) -> Result<(), String> {
        // Get the universal manifest
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
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

    pub async fn check_for_updates(&mut self, http_client: &CachedHttpClient, presets: &[Preset]) -> Result<bool, String> {
        // Check modpack updates using semantic version comparison
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        // Use the compare_versions function for modpack version
        let modpack_update = match crate::compare_versions(&universal_manifest.modpack_version, &self.universal_version) {
            std::cmp::Ordering::Greater => {
                debug!("Modpack update available: {} -> {}", self.universal_version, universal_manifest.modpack_version);
                true
            },
            _ => false
        };
        
        // Check preset updates
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
        
        // Update both flags
        self.update_available = modpack_update || preset_update;
        self.preset_update_available = preset_update;
        
        self.save()?;
        Ok(self.update_available)
    }

    pub async fn check_preset_updates(&self, presets: &[Preset]) -> Option<String> {
        if let Some(base_preset_id) = &self.base_preset_id {
            if let Some(current_preset) = presets.iter().find(|p| p.id == *base_preset_id) {
                // Check if preset version has changed
                if let (Some(current_version), Some(base_version)) = 
                    (&current_preset.preset_version, &self.base_preset_version) {
                    if current_version != base_version {
                        return Some(format!(
                            "Preset '{}' has been updated from {} to {}",
                            current_preset.name, base_version, current_version
                        ));
                    }
                }
            }
        }
        None
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
        self.modified = false;
        self.last_used = chrono::Utc::now();
        self.save()
    }

    pub async fn install_or_update(&self, http_client: &crate::CachedHttpClient) -> Result<(), String> {
        self.install_or_update_with_progress(http_client, || {}).await
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
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        // Update installation state
        self.installed = true;
        self.update_available = false;
        self.modified = false;
        self.universal_version = universal_manifest.modpack_version;
        self.last_used = chrono::Utc::now();
        
        self.save()
    }

    pub fn has_preset_modifications(&self, presets: &[crate::preset::Preset]) -> bool {
        if let Some(base_preset_id) = &self.base_preset_id {
            if let Some(preset) = presets.iter().find(|p| p.id == *base_preset_id) {
                // Check if current features differ from preset's original features
                let preset_features = &preset.enabled_features;
                let current_features = &self.enabled_features;
                
                // Simple comparison - if they're different, user has modified
                preset_features != current_features
            } else {
                // Preset not found, consider it modified
                true
            }
        } else {
            // No preset selected, so it's custom
            false
        }
    }

    // Apply this preset to an installation, returning the list of enabled features
    pub fn apply_preset_with_tracking(&mut self, preset: &crate::preset::Preset) {
        debug!("Applying preset '{}' to installation '{}'", preset.name, self.name);
        
        // Store previous state for comparison
        let previous_features = self.enabled_features.clone();
        let previous_preset = self.base_preset_id.clone();
        
        // Apply the preset
        self.base_preset_id = Some(preset.id.clone());
        self.base_preset_version = preset.preset_version.clone();
        self.enabled_features = preset.enabled_features.clone();
        
        // NEW: Update selection tracking
        self.selected_preset_id = Some(preset.id.clone());
        self.pending_features = preset.enabled_features.clone();
        self.is_custom_configuration = false;
        
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
        debug!("Preset tracking: base_preset_id = {:?}, base_preset_version = {:?}", 
               self.base_preset_id, self.base_preset_version);
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
        
        // NEW: Update selection tracking
        self.selected_preset_id = None;
        self.is_custom_configuration = true;
        
        // Keep existing features but clear change tracking
        self.custom_features.clear();
        self.removed_features.clear();
        
        self.modified = true;
        
        debug!("Switched to custom - current features: {:?}", self.enabled_features);
        debug!("Preset tracking cleared: base_preset_id = None");
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
        
        // NEW: Update pending features
        self.pending_features = self.enabled_features.clone();
        
        // Track changes relative to base preset if we have one
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
                
                debug!("Feature tracking for preset '{}': custom={:?}, removed={:?}", 
                       base_preset_id, self.custom_features, self.removed_features);
            }
        }
        // If no preset is selected (custom mode), no special tracking needed
        
        self.modified = true;
        
        debug!("Feature toggle complete - enabled: {:?}", self.enabled_features);
    }

    // Enhanced method to properly initialize from universal manifest  
    pub async fn initialize_with_universal_defaults(&mut self, http_client: &crate::CachedHttpClient) -> Result<(), String> {
        debug!("Initializing installation '{}' with universal defaults", self.name);
        
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        let mut features = vec!["default".to_string()];
        
        // Add all default-enabled components from universal manifest
        for component in &universal_manifest.mods {
            if component.default_enabled && component.id != "default" && !features.contains(&component.id) {
                features.push(component.id.clone());
                debug!("Added default mod: {}", component.id);
            }
        }
        
        for component in &universal_manifest.shaderpacks {
            if component.default_enabled && component.id != "default" && !features.contains(&component.id) {
                features.push(component.id.clone());
                debug!("Added default shaderpack: {}", component.id);
            }
        }
        
        for component in &universal_manifest.resourcepacks {
            if component.default_enabled && component.id != "default" && !features.contains(&component.id) {
                features.push(component.id.clone());
                debug!("Added default resourcepack: {}", component.id);
            }
        }
        
        for include in &universal_manifest.include {
            if include.default_enabled && !include.id.is_empty() && include.id != "default" 
               && !features.contains(&include.id) {
                features.push(include.id.clone());
                debug!("Added default include: {}", include.id);
            }
        }
        
        for remote in &universal_manifest.remote_include {
            if remote.default_enabled && remote.id != "default" 
               && !features.contains(&remote.id) {
                features.push(remote.id.clone());
                debug!("Added default remote include: {}", remote.id);
            }
        }
        
        // Set features and ensure we're in custom mode for new installations
        self.enabled_features = features.clone();
        self.pending_features = features.clone();
        self.pre_install_features = features;
        self.base_preset_id = None; // Start as custom
        self.base_preset_version = None;
        self.selected_preset_id = None;
        self.is_custom_configuration = true;
        self.custom_features.clear();
        self.removed_features.clear();
        
        debug!("Initialized installation with {} default features: {:?}", 
               self.enabled_features.len(), self.enabled_features);
        
        Ok(())
    }

    // Check if the current configuration matches a specific preset exactly
    pub fn matches_preset(&self, preset: &crate::preset::Preset) -> bool {
        // Simple comparison - does our enabled_features exactly match the preset's?
        let mut our_features = self.enabled_features.clone();
        let mut preset_features = preset.enabled_features.clone();
        
        our_features.sort();
        preset_features.sort();
        
        our_features == preset_features
    }

    pub fn get_effective_preset_id(&self, presets: &[crate::preset::Preset]) -> Option<String> {
        // If we have a base preset ID and no modifications, return it
        if let Some(base_id) = &self.base_preset_id {
            if self.custom_features.is_empty() && self.removed_features.is_empty() {
                return Some(base_id.clone());
            }
            // If we have modifications but still based on a preset, return the base
            return Some(base_id.clone());
        }
        
        // If no base preset, check if we exactly match any preset
        for preset in presets {
            if self.matches_preset(preset) {
                debug!("Installation exactly matches preset: {}", preset.id);
                return Some(preset.id.clone());
            }
        }
        
        // No preset match - this is custom
        None
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

    // Method to initialize enabled features based on universal manifest
    pub async fn initialize_default_features(&mut self, http_client: &crate::CachedHttpClient) -> Result<(), String> {
        let universal_manifest = crate::universal::load_universal_manifest(http_client, None).await
            .map_err(|e| format!("Failed to load universal manifest: {}", e))?;
        
        let mut features = vec!["default".to_string()];
        
        debug!("Initializing default features from universal manifest");
        
        // Add all default-enabled components AND ensure all their IDs are in enabled_features
        for component in &universal_manifest.mods {
            if component.default_enabled || component.id == "default" {
                if !features.contains(&component.id) {
                    features.push(component.id.clone());
                    debug!("Added default mod: {}", component.id);
                }
            }
        }
        
        for component in &universal_manifest.shaderpacks {
            if component.default_enabled || component.id == "default" {
                if !features.contains(&component.id) {
                    features.push(component.id.clone());
                    debug!("Added default shaderpack: {}", component.id);
                }
            }
        }
        
        for component in &universal_manifest.resourcepacks {
            if component.default_enabled || component.id == "default" {
                if !features.contains(&component.id) {
                    features.push(component.id.clone());
                    debug!("Added default resourcepack: {}", component.id);
                }
            }
        }
        
        for include in &universal_manifest.include {
            if (include.default_enabled || include.id == "default") && !include.id.is_empty() {
                if !features.contains(&include.id) {
                    features.push(include.id.clone());
                    debug!("Added default include: {}", include.id);
                }
            }
        }
        
        for remote in &universal_manifest.remote_include {
            if remote.default_enabled || remote.id == "default" {
                if !features.contains(&remote.id) {
                    features.push(remote.id.clone());
                    debug!("Added default remote include: {}", remote.id);
                }
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

    // Helper to check if this installation should show update button
    pub fn should_show_update_button(&self) -> bool {
        if !self.installed {
            // Not installed yet - show install button
            false
        } else if self.update_available || self.preset_update_available {
            // Updates available - show update button
            true
        } else if self.modified {
            // User made changes - show modify button  
            true
        } else {
            // Installed and up-to-date with no changes
            false
        }
    }

    // Get the appropriate button label
    pub fn get_action_button_label(&self) -> &'static str {
        if !self.installed {
            "INSTALL"
        } else if self.update_available || self.preset_update_available {
            "UPDATE"
        } else if self.modified {
            "MODIFY"
        } else {
            "INSTALLED"
        }
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
    
    let mut installation: Installation = serde_json::from_str(&config_json)
        .map_err(|e| format!("Failed to parse installation config: {}", e))?;
    
    // MIGRATION: Initialize new fields if they don't exist
    if installation.selected_preset_id.is_none() && installation.base_preset_id.is_some() {
        installation.selected_preset_id = installation.base_preset_id.clone();
    }
    
    if installation.pre_install_features.is_empty() && !installation.enabled_features.is_empty() {
        installation.pre_install_features = installation.enabled_features.clone();
    }
    
    if installation.installed_features.is_empty() && installation.installed {
        installation.installed_features = installation.enabled_features.clone();
    }
    
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
