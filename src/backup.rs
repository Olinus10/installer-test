use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Write};
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use zip::{ZipWriter, CompressionMethod};
use std::collections::HashMap;

/// Types of backups that can be created
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupType {
    Manual,
    PreUpdate,
    PreInstall,
    Scheduled,
}

/// Dynamic folder/file item discovered in installation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupItem {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size_bytes: u64,
    pub file_count: Option<usize>, // Only for directories
    pub description: Option<String>, // User-friendly description
}

/// Configuration for what to include in backups - now dynamic
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupConfig {
    pub selected_items: Vec<String>, // Paths relative to installation root
    pub compress_backups: bool,
    pub max_backups: usize,
    pub include_hidden_files: bool,
    pub exclude_patterns: Vec<String>, // Glob patterns to exclude
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            selected_items: vec![
                "mods".to_string(),
                "config".to_string(),
                "resourcepacks".to_string(),
                "shaderpacks".to_string(),
            ],
            compress_backups: true,
            max_backups: 10,
            include_hidden_files: false,
            exclude_patterns: vec![
                "*.log".to_string(),
                "crash-reports".to_string(),
                "logs".to_string(),
                "temp".to_string(),
                "cache".to_string(),
            ],
        }
    }
}

/// Metadata about a backup
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupMetadata {
    pub id: String,
    pub description: String,
    pub backup_type: BackupType,
    pub created_at: DateTime<Utc>,
    pub modpack_version: String,
    pub enabled_features: Vec<String>,
    pub file_count: usize,
    pub size_bytes: u64,
    pub included_items: Vec<String>, // List of backed up items
    pub config: BackupConfig,
}

impl BackupMetadata {
    pub fn age_description(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.created_at);
        
        if duration.num_days() > 0 {
            format!("{} days ago", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{} hours ago", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{} minutes ago", duration.num_minutes())
        } else {
            "Just now".to_string()
        }
    }
    
    pub fn formatted_size(&self) -> String {
        format_bytes(self.size_bytes)
    }
}

/// Progress tracking for backup operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupProgress {
    pub current_file: String,
    pub files_processed: usize,
    pub total_files: usize,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub current_operation: String, // e.g., "Scanning files", "Creating archive"
}

/// Rollback manager for emergency recovery
#[derive(Debug, Clone)]
pub struct RollbackManager {
    pub installation: crate::installation::Installation,
}

impl RollbackManager {
    pub fn new(installation: crate::installation::Installation) -> Self {
        Self { installation }
    }
    
    pub fn get_rollback_options(&self) -> Result<Vec<RollbackOption>, String> {
        let backups = self.installation.list_available_backups()?;
        let mut options = Vec::new();
        
        for backup in &backups {
            let is_recommended = backup.backup_type == BackupType::PreUpdate || 
                               backup.backup_type == BackupType::Manual;
            
            options.push(RollbackOption {
                backup_id: backup.id.clone(),
                description: backup.description.clone(),
                modpack_version: backup.modpack_version.clone(),
                created_at: backup.created_at,
                size: backup.size_bytes,
                is_recommended,
            });
        }
        
        options.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(options)
    }
    
    pub async fn rollback_to_last_working(&mut self) -> Result<(), String> {
        let options = self.get_rollback_options()?;
        
        if let Some(last_working) = options.first() {
            self.rollback_to_backup(&last_working.backup_id).await
        } else {
            Err("No backups available for rollback".to_string())
        }
    }
    
    pub async fn rollback_to_backup(&mut self, backup_id: &str) -> Result<(), String> {
        let safety_config = BackupConfig::default();
        let safety_description = format!("Safety backup before rollback to {}", backup_id);
        
        self.installation.create_backup_dynamic(
            BackupType::PreUpdate,
            &safety_config,
            safety_description,
            None::<fn(BackupProgress)>,
        ).await?;
        
        self.installation.restore_from_backup(backup_id).await?;
        Ok(())
    }
}

/// Rollback option for the UI
#[derive(Debug, Clone, PartialEq)]
pub struct RollbackOption {
    pub backup_id: String,
    pub description: String,
    pub modpack_version: String,
    pub created_at: DateTime<Utc>,
    pub size: u64,
    pub is_recommended: bool,
}

/// Enhanced backup discovery system
impl crate::installation::Installation {
    /// Discover all backup-able items in the installation directory
    pub fn discover_backup_items(&self) -> Result<Vec<BackupItem>, String> {
        let installation_path = &self.installation_path;
        
        if !installation_path.exists() {
            return Ok(Vec::new());
        }
        
        let mut items = Vec::new();
        
        match fs::read_dir(installation_path) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let name = entry.file_name().to_string_lossy().to_string();
                        
                        // Skip system/hidden files unless explicitly requested
                        if name.starts_with('.') || name.starts_with('~') {
                            continue;
                        }
                        
                        // Skip temporary files and common system files
                        if matches!(name.as_str(), "desktop.ini" | "Thumbs.db" | ".DS_Store") {
                            continue;
                        }
                        
                        let is_directory = path.is_dir();
                        let relative_path = path.strip_prefix(installation_path)
                            .map_err(|e| format!("Failed to get relative path: {}", e))?
                            .to_path_buf();
                        
                        let size_bytes = if is_directory {
                            calculate_directory_size(&path).unwrap_or(0)
                        } else {
                            fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
                        };
                        
                        let file_count = if is_directory {
                            Some(count_files_recursive(&path).unwrap_or(0))
                        } else {
                            None
                        };
                        
                        // Generate user-friendly descriptions
                        let description = get_item_description(&name, is_directory);
                        
                        items.push(BackupItem {
                            name,
                            path: relative_path,
                            is_directory,
                            size_bytes,
                            file_count,
                            description,
                        });
                    }
                }
            },
            Err(e) => return Err(format!("Failed to read installation directory: {}", e)),
        }
        
        // Sort items: directories first, then by name
        items.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        Ok(items)
    }
    
    /// Enhanced backup creation with dynamic item selection
    pub async fn create_backup_dynamic<F>(
        &self,
        backup_type: crate::backup::BackupType,
        config: &BackupConfig,
        description: String,
        progress_callback: Option<F>,
    ) -> Result<BackupMetadata, String>
    where
        F: Fn(BackupProgress) + Send + Sync + Clone + 'static,
    {
        use chrono::Utc;
        use uuid::Uuid;
        
        let backup_id = Uuid::new_v4().to_string();
        let backups_dir = self.get_backups_dir();
        let backup_dir = backups_dir.join(&backup_id);
        
        // Create backup directory
        std::fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;
        
        debug!("Creating dynamic backup {} for installation {}", backup_id, self.name);
        
        // Collect items to backup based on configuration
        let mut items_to_backup = Vec::new();
        let mut total_files = 0;
        let mut total_bytes = 0;
        
        for item_path in &config.selected_items {
            let full_path = self.installation_path.join(item_path);
            if full_path.exists() {
                items_to_backup.push((item_path.clone(), full_path.clone()));
                
                if full_path.is_dir() {
                    total_files += count_files_recursive(&full_path).unwrap_or(0);
                    total_bytes += calculate_directory_size(&full_path).unwrap_or(0);
                } else {
                    total_files += 1;
                    total_bytes += fs::metadata(&full_path).map(|m| m.len()).unwrap_or(0);
                }
            }
        }
        
        debug!("Backup will include {} items ({} files, {} bytes)", 
               items_to_backup.len(), total_files, total_bytes);
        
        let mut files_processed = 0;
        let mut bytes_processed = 0;
        
        // Notify progress of scanning completion
        if let Some(ref callback) = progress_callback {
            callback(BackupProgress {
                current_file: "Preparing backup...".to_string(),
                files_processed: 0,
                total_files,
                bytes_processed: 0,
                total_bytes,
                current_operation: "Scanning files".to_string(),
            });
        }
        
        if config.compress_backups {
            // Create ZIP archive
            let archive_path = backup_dir.join("backup.zip");
            let temp_dir = backup_dir.join("temp");
            std::fs::create_dir_all(&temp_dir)
                .map_err(|e| format!("Failed to create temp directory: {}", e))?;
            
            // Copy selected items to temp directory
            for (item_name, source_path) in &items_to_backup {
                let dest_path = temp_dir.join(item_name);
                
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                }
                
                self.copy_item_with_progress(
                    source_path,
                    &dest_path,
                    &mut files_processed,
                    total_files,
                    &mut bytes_processed,
                    &progress_callback,
                    &config.exclude_patterns,
                )?;
            }
            
            // Update progress for compression phase
            if let Some(ref callback) = progress_callback {
                callback(BackupProgress {
                    current_file: "Creating archive...".to_string(),
                    files_processed,
                    total_files,
                    bytes_processed,
                    total_bytes,
                    current_operation: "Compressing files".to_string(),
                });
            }
            
            // Create ZIP archive
            let final_size = create_zip_archive(
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
            for (item_name, source_path) in &items_to_backup {
                let dest_path = backup_dir.join(item_name);
                
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                }
                
                self.copy_item_with_progress(
                    source_path,
                    &dest_path,
                    &mut files_processed,
                    total_files,
                    &mut bytes_processed,
                    &progress_callback,
                    &config.exclude_patterns,
                )?;
            }
        }
        
        // Create metadata with included items list
        let included_items = config.selected_items.clone();
        
        let metadata = BackupMetadata {
            id: backup_id.clone(),
            description,
            backup_type,
            created_at: Utc::now(),
            modpack_version: self.universal_version.clone(),
            enabled_features: self.enabled_features.clone(),
            file_count: files_processed,
            size_bytes: bytes_processed,
            included_items,
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
        
        // Final progress update
        if let Some(ref callback) = progress_callback {
            callback(BackupProgress {
                current_file: "Backup completed!".to_string(),
                files_processed,
                total_files,
                bytes_processed,
                total_bytes,
                current_operation: "Finished".to_string(),
            });
        }
        
        info!("Successfully created dynamic backup {} for installation {}", backup_id, self.name);
        Ok(metadata)
    }
    
    /// Helper method to copy a single item (file or directory) with progress and exclusion patterns
    fn copy_item_with_progress<F>(
        &self,
        source: &Path,
        dest: &Path,
        files_processed: &mut usize,
        total_files: usize,
        bytes_processed: &mut u64,
        progress_callback: &Option<F>,
        exclude_patterns: &[String],
    ) -> Result<(), String>
    where
        F: Fn(BackupProgress) + Clone,
    {
        // Check if this item should be excluded
        if should_exclude_path(source, exclude_patterns) {
            debug!("Excluding path: {:?}", source);
            return Ok(());
        }
        
        if source.is_file() {
            // Copy single file
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }
            
            std::fs::copy(source, dest)
                .map_err(|e| format!("Failed to copy file {:?}: {}", source, e))?;
                
            let file_size = std::fs::metadata(dest)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .len();
                
            *files_processed += 1;
            *bytes_processed += file_size;
            
            if let Some(callback) = progress_callback {
                callback(BackupProgress {
                    current_file: source.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    files_processed: *files_processed,
                    total_files,
                    bytes_processed: *bytes_processed,
                    total_bytes: 0,
                    current_operation: "Copying files".to_string(),
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
            
            self.copy_item_with_progress(
                &source_path,
                &dest_path,
                files_processed,
                total_files,
                bytes_processed,
                progress_callback,
                exclude_patterns,
            )?;
        }
        
        Ok(())
    }
}

/// Check if a path should be excluded based on patterns
    fn should_exclude_path(path: &Path, exclude_patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        
        for pattern in exclude_patterns {
            // Simple glob-like matching
            if pattern.contains('*') {
                let pattern_without_star = pattern.replace('*', "");
                if path_str.contains(&pattern_without_star) || file_name.contains(&pattern_without_star) {
                    return true;
                }
            } else if path_str.ends_with(pattern) || file_name.as_ref() == pattern { // Fix comparison
                return true;
            }
        }
        
        false
    }
}

/// Generate user-friendly descriptions for backup items
fn get_item_description(name: &str, is_directory: bool) -> Option<String> {
    let description = match name.to_lowercase().as_str() {
        "mods" => "Mod files and configurations",
        "config" => "Game and mod configuration files",
        "resourcepacks" => "Resource pack files",
        "shaderpacks" => "Shader pack files", 
        "saves" => "World save files",
        "screenshots" => "Screenshot images",
        "logs" => "Game and launcher log files",
        "crash-reports" => "Crash report files",
        "wynntils" => "Wynntils mod configuration and data",
        "options.txt" => "Minecraft game options",
        "servers.dat" => "Multiplayer server list",
        "usercache.json" => "User cache data",
        "usernamecache.json" => "Username cache data",
        _ => {
            if is_directory {
                "Custom directory"
            } else if name.ends_with(".json") {
                "Configuration file"
            } else if name.ends_with(".txt") {
                "Text file"
            } else if name.ends_with(".jar") {
                "Java application file"
            } else {
                "Custom file"
            }
        }
    };
    
    Some(description.to_string())
}

/// Format bytes in a human-readable way
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Calculate directory size recursively
pub fn calculate_directory_size(path: &Path) -> io::Result<u64> {
    let mut total_size = 0;
    
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            
            if entry_path.is_file() {
                total_size += entry.metadata()?.len();
            } else if entry_path.is_dir() {
                total_size += calculate_directory_size(&entry_path)?;
            }
        }
    }
    
    Ok(total_size)
}

/// Count files recursively in a directory
pub fn count_files_recursive(path: &Path) -> io::Result<usize> {
    let mut count = 0;
    
    if path.is_file() {
        return Ok(1);
    }
    
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            
            if entry_path.is_file() {
                count += 1;
            } else if entry_path.is_dir() {
                count += count_files_recursive(&entry_path)?;
            }
        }
    }
    
    Ok(count)
}

/// Create a ZIP archive from a directory
pub fn create_zip_archive<F>(
    source_dir: &Path,
    zip_path: &Path,
    progress_callback: Option<&F>,
) -> Result<u64, io::Error>
where
    F: Fn(BackupProgress),
{
    let file = fs::File::create(zip_path)?;
    let mut zip = ZipWriter::new(file);
    
    let total_files = count_files_recursive(source_dir)?;
    let mut files_processed = 0;
    let mut bytes_processed = 0;
    
    add_directory_to_zip(
        &mut zip,
        source_dir,
        "",
        &mut files_processed,
        total_files,
        &mut bytes_processed,
        &progress_callback,
    )?;
    
    let final_size = zip.finish()?.metadata()?.len();
    Ok(final_size)
}

fn add_directory_to_zip<F, W: Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    source_dir: &Path,
    prefix: &str,
    files_processed: &mut usize,
    total_files: usize,
    bytes_processed: &mut u64,
    progress_callback: &Option<&F>,
) -> Result<(), io::Error>
where
    F: Fn(BackupProgress),
{
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        
        let full_name = if prefix.is_empty() {
            name_str.to_string()
        } else {
            format!("{}/{}", prefix, name_str)
        };
        
        if path.is_file() {
            let file_data = fs::read(&path)?;
            let file_size = file_data.len() as u64;
            
            let options = zip::write::FileOptions::<()>::default()
                .compression_method(CompressionMethod::Deflated);
            zip.start_file(&full_name, options)?;
            zip.write_all(&file_data)?;
            
            *files_processed += 1;
            *bytes_processed += file_size;
            
            if let Some(callback) = progress_callback {
                callback(BackupProgress {
                    current_file: full_name,
                    files_processed: *files_processed,
                    total_files,
                    bytes_processed: *bytes_processed,
                    total_bytes: 0,
                    current_operation: "Compressing files".to_string(),
                });
            }
        } else if path.is_dir() {
            add_directory_to_zip(
                zip,
                &path,
                &full_name,
                files_processed,
                total_files,
                bytes_processed,
                progress_callback,
            )?;
        }
    }
    
    Ok(())
}

/// Extract a ZIP archive to a directory
pub fn extract_zip_archive(zip_path: &Path, destination: &Path) -> Result<(), io::Error> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = destination.join(file.name());
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    
    Ok(())
}
