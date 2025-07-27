use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Write};
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use zip::{ZipWriter, CompressionMethod};
use std::collections::HashMap;

/// Enhanced backup item discovery with better file/folder scanning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupItem {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size_bytes: u64,
    pub file_count: Option<usize>, // Only for directories
    pub description: Option<String>, // User-friendly description
    pub children: Option<Vec<BackupItem>>, // NEW: For nested items
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

/// Configuration for what to include in backups
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

/// Types of backups that can be created
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupType {
    Manual,
    PreUpdate,
    PreInstall,
    Scheduled,
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
        
        self.installation.create_backup(
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

/// Create a ZIP archive from a directory
pub fn create_zip_archive<F>(
    source_dir: &std::path::Path,
    zip_path: &std::path::Path,
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
    source_dir: &std::path::Path,
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
pub fn extract_zip_archive(zip_path: &std::path::Path, destination: &std::path::Path) -> Result<(), io::Error> {
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

impl BackupItem {
    /// Create a new backup item with enhanced metadata
    pub fn new(
        name: String,
        path: PathBuf,
        is_directory: bool,
        include_children: bool,
    ) -> Result<Self, String> {
        let full_path = path.clone();
        
        let (size_bytes, file_count) = if is_directory {
            let size = calculate_directory_size(&full_path)?;
            let count = count_files_recursive(&full_path)?;
            (size, Some(count))
        } else {
            let size = fs::metadata(&full_path)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .len();
            (size, None)
        };
        
        let description = get_item_description(&name, is_directory);
        
        let children = if include_children && is_directory {
            Some(discover_children(&full_path)?)
        } else {
            None
        };
        
        Ok(BackupItem {
            name,
            path,
            is_directory,
            size_bytes,
            file_count,
            description,
            children,
        })
    }
    
    /// Get a flat list of all items including children
    pub fn flatten(&self) -> Vec<BackupItem> {
        let mut items = vec![self.clone()];
        
        if let Some(children) = &self.children {
            for child in children {
                items.extend(child.flatten());
            }
        }
        
        items
    }
    
    /// Check if this item should be included in backup based on patterns
    pub fn should_include(&self, include_patterns: &[String], exclude_patterns: &[String]) -> bool {
        let path_str = self.path.to_string_lossy().to_string();
        let name = &self.name;
        
        // Check exclude patterns first
        for pattern in exclude_patterns {
            if matches_pattern(&path_str, pattern) || matches_pattern(name, pattern) {
                return false;
            }
        }
        
        // If no include patterns specified, include everything not excluded
        if include_patterns.is_empty() {
            return true;
        }
        
        // Check include patterns
        for pattern in include_patterns {
            if matches_pattern(&path_str, pattern) || matches_pattern(name, pattern) {
                return true;
            }
        }
        
        false
    }
}

/// Enhanced backup discovery function
pub fn discover_installation_items(installation_path: &Path, max_depth: usize) -> Result<Vec<BackupItem>, String> {
    let mut items = Vec::new();
    
    debug!("Discovering backup items in: {:?}", installation_path);
    
    if !installation_path.exists() {
        return Err("Installation path does not exist".to_string());
    }
    
    let entries = fs::read_dir(installation_path)
        .map_err(|e| format!("Failed to read installation directory: {}", e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        
        // Skip hidden files unless specifically allowed
        if name.starts_with('.') && !is_allowed_hidden_item(&name) {
            continue;
        }
        
        // Skip known temporary/system files
        if should_skip_item(&name) {
            continue;
        }
        
        let relative_path = path.strip_prefix(installation_path)
            .unwrap_or(&path)
            .to_path_buf();
        
        let is_directory = path.is_dir();
        
        match BackupItem::new(name, relative_path, is_directory, max_depth > 0) {
            Ok(item) => {
                debug!("Discovered item: {} ({})", item.name, 
                       if item.is_directory { "directory" } else { "file" });
                items.push(item);
            },
            Err(e) => {
                warn!("Failed to create backup item for {:?}: {}", path, e);
            }
        }
    }
    
    // Sort by priority and name
    items.sort_by(|a, b| {
        let a_priority = get_folder_priority(&a.name);
        let b_priority = get_folder_priority(&b.name);
        
        match a_priority.cmp(&b_priority) {
            std::cmp::Ordering::Equal => a.name.cmp(&b.name),
            other => other,
        }
    });
    
    debug!("Discovered {} backup items", items.len());
    Ok(items)
}

/// Discover children of a directory (one level deep)
fn discover_children(dir_path: &Path) -> Result<Vec<BackupItem>, String> {
    let mut children = Vec::new();
    
    if !dir_path.is_dir() {
        return Ok(children);
    }
    
    let entries = fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        
        if should_skip_item(&name) {
            continue;
        }
        
        let is_directory = path.is_dir();
        
        match BackupItem::new(name, path, is_directory, false) {
            Ok(item) => children.push(item),
            Err(e) => warn!("Failed to create child item: {}", e),
        }
    }
    
    // Limit children to prevent UI overload
    children.sort_by(|a, b| a.name.cmp(&b.name));
    children.truncate(50); // Limit to 50 children per directory
    
    Ok(children)
}

/// Enhanced backup creation with better progress tracking
impl crate::installation::Installation {
    /// Create backup with improved progress and file discovery
    pub async fn create_backup_enhanced<F>(
        &self,
        backup_type: crate::backup::BackupType,
        config: &crate::backup::BackupConfig,
        description: String,
        progress_callback: Option<F>,
    ) -> Result<crate::backup::BackupMetadata, String>
    where
        F: Fn(crate::backup::BackupProgress) + Send + Sync + Clone + 'static,
    {
        use uuid::Uuid;
        
        let backup_id = Uuid::new_v4().to_string();
        let backups_dir = self.get_backups_dir();
        let backup_dir = backups_dir.join(&backup_id);
        
        // Create backup directory
        fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;
        
        debug!("Creating enhanced backup {} for installation {}", backup_id, self.name);
        
        // Discover all available items first
        let all_items = discover_installation_items(&self.installation_path, 1)?;
        
        // Filter items based on configuration
        let mut items_to_backup = Vec::new();
        let mut total_files = 0;
        let mut total_bytes = 0;
        
        for item in &all_items {
            let item_path_str = item.path.to_string_lossy().to_string();
            
            // Check if this item is selected for backup
            if config.selected_items.contains(&item_path_str) {
                items_to_backup.push(item);
                total_files += item.file_count.unwrap_or(1);
                total_bytes += item.size_bytes;
                
                debug!("Selected for backup: {} ({} bytes)", item.name, item.size_bytes);
            }
        }
        
        if items_to_backup.is_empty() {
            return Err("No items selected for backup".to_string());
        }
        
        debug!("Backup will include {} items ({} files, {} bytes)", 
               items_to_backup.len(), total_files, total_bytes);
        
        let mut files_processed = 0;
        let mut bytes_processed = 0;
        
        // Initial progress
        if let Some(ref callback) = progress_callback {
            callback(crate::backup::BackupProgress {
                current_file: "Preparing backup...".to_string(),
                files_processed: 0,
                total_files,
                bytes_processed: 0,
                total_bytes,
                current_operation: "Scanning files".to_string(),
            });
        }
        
        if config.compress_backups {
            // Create compressed backup
            let archive_path = backup_dir.join("backup.zip");
            
            self.create_compressed_backup(
                &items_to_backup,
                &archive_path,
                &mut files_processed,
                total_files,
                &mut bytes_processed,
                &progress_callback,
                &config.exclude_patterns,
            ).await?;
            
            bytes_processed = fs::metadata(&archive_path)
                .map_err(|e| format!("Failed to get archive size: {}", e))?
                .len();
        } else {
            // Create uncompressed backup
            self.create_uncompressed_backup(
                &items_to_backup,
                &backup_dir,
                &mut files_processed,
                total_files,
                &mut bytes_processed,
                &progress_callback,
                &config.exclude_patterns,
            ).await?;
        }
        
        // Create metadata
        let included_items = items_to_backup.iter()
            .map(|item| item.path.to_string_lossy().to_string())
            .collect();
        
        let metadata = crate::backup::BackupMetadata {
            id: backup_id.clone(),
            description,
            backup_type,
            created_at: chrono::Utc::now(),
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
        fs::write(&metadata_path, metadata_json)
            .map_err(|e| format!("Failed to write metadata: {}", e))?;
        
        // Clean up old backups
        self.cleanup_old_backups(config.max_backups)?;
        
        // Final progress update
        if let Some(ref callback) = progress_callback {
            callback(crate::backup::BackupProgress {
                current_file: "Backup completed!".to_string(),
                files_processed,
                total_files,
                bytes_processed,
                total_bytes,
                current_operation: "Finished".to_string(),
            });
        }
        
        info!("Successfully created enhanced backup {} for installation {}", backup_id, self.name);
        Ok(metadata)
    }
    
    /// Create compressed backup using ZIP
    async fn create_compressed_backup<F>(
        &self,
        items: &[&BackupItem],
        archive_path: &Path,
        files_processed: &mut usize,
        total_files: usize,
        bytes_processed: &mut u64,
        progress_callback: &Option<F>,
        exclude_patterns: &[String],
    ) -> Result<(), String>
    where
        F: Fn(crate::backup::BackupProgress) + Clone,
    {
        let file = fs::File::create(archive_path)
            .map_err(|e| format!("Failed to create archive: {}", e))?;
        let mut zip = ZipWriter::new(file);
        
        for item in items {
            let source_path = self.installation_path.join(&item.path);
            
            if source_path.is_file() {
                self.add_file_to_zip(
                    &mut zip,
                    &source_path,
                    &item.path.to_string_lossy(),
                    files_processed,
                    total_files,
                    bytes_processed,
                    progress_callback,
                )?;
            } else if source_path.is_dir() {
                self.add_directory_to_zip(
                    &mut zip,
                    &source_path,
                    &item.path.to_string_lossy(),
                    files_processed,
                    total_files,
                    bytes_processed,
                    progress_callback,
                    exclude_patterns,
                )?;
            }
        }
        
        zip.finish()
            .map_err(|e| format!("Failed to finalize archive: {}", e))?;
        
        Ok(())
    }
    
    /// Create uncompressed backup by copying files
    async fn create_uncompressed_backup<F>(
        &self,
        items: &[&BackupItem],
        backup_dir: &Path,
        files_processed: &mut usize,
        total_files: usize,
        bytes_processed: &mut u64,
        progress_callback: &Option<F>,
        exclude_patterns: &[String],
    ) -> Result<(), String>
    where
        F: Fn(crate::backup::BackupProgress) + Clone,
    {
        for item in items {
            let source_path = self.installation_path.join(&item.path);
            let dest_path = backup_dir.join(&item.path);
            
            if should_exclude_path(&source_path, exclude_patterns) {
                continue;
            }
            
            if source_path.is_file() {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }
                
                fs::copy(&source_path, &dest_path)
                    .map_err(|e| format!("Failed to copy file: {}", e))?;
                
                let file_size = fs::metadata(&dest_path)
                    .map_err(|e| format!("Failed to get file size: {}", e))?
                    .len();
                
                *files_processed += 1;
                *bytes_processed += file_size;
                
                if let Some(callback) = progress_callback {
                    callback(crate::backup::BackupProgress {
                        current_file: item.name.clone(),
                        files_processed: *files_processed,
                        total_files,
                        bytes_processed: *bytes_processed,
                        total_bytes: 0,
                        current_operation: "Copying files".to_string(),
                    });
                }
            } else if source_path.is_dir() {
                self.copy_directory_recursive(
                    &source_path,
                    &dest_path,
                    files_processed,
                    total_files,
                    bytes_processed,
                    progress_callback,
                    exclude_patterns,
                )?;
            }
        }
        
        Ok(())
    }
    
    /// Helper method to add file to ZIP
    fn add_file_to_zip<F, W: Write + io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        source_path: &Path,
        archive_path: &str,
        files_processed: &mut usize,
        total_files: usize,
        bytes_processed: &mut u64,
        progress_callback: &Option<F>,
    ) -> Result<(), String>
    where
        F: Fn(crate::backup::BackupProgress) + Clone,
    {
        let file_data = fs::read(source_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        let file_size = file_data.len() as u64;
        
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(CompressionMethod::Deflated);
        
        zip.start_file(archive_path, options)
            .map_err(|e| format!("Failed to start file in zip: {}", e))?;
        zip.write_all(&file_data)
            .map_err(|e| format!("Failed to write file to zip: {}", e))?;
        
        *files_processed += 1;
        *bytes_processed += file_size;
        
        if let Some(callback) = progress_callback {
            callback(crate::backup::BackupProgress {
                current_file: source_path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                files_processed: *files_processed,
                total_files,
                bytes_processed: *bytes_processed,
                total_bytes: 0,
                current_operation: "Compressing files".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Helper method to add directory to ZIP recursively
    fn add_directory_to_zip<F, W: Write + io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        source_dir: &Path,
        archive_prefix: &str,
        files_processed: &mut usize,
        total_files: usize,
        bytes_processed: &mut u64,
        progress_callback: &Option<F>,
        exclude_patterns: &[String],
    ) -> Result<(), String>
    where
        F: Fn(crate::backup::BackupProgress) + Clone,
    {
        let entries = fs::read_dir(source_dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            let file_name = entry.file_name();
            let name_string = file_name.to_string_lossy().to_string();
            
            if should_exclude_path(&path, exclude_patterns) {
                continue;
            }
            
            let archive_path = if archive_prefix.is_empty() {
                name_string
            } else {
                format!("{}/{}", archive_prefix, name)
            };
            
            if path.is_file() {
                self.add_file_to_zip(
                    zip,
                    &path,
                    &archive_path,
                    files_processed,
                    total_files,
                    bytes_processed,
                    progress_callback,
                )?;
            } else if path.is_dir() {
                self.add_directory_to_zip(
                    zip,
                    &path,
                    &archive_path,
                    files_processed,
                    total_files,
                    bytes_processed,
                    progress_callback,
                    exclude_patterns,
                )?;
            }
        }
        
        Ok(())
    }
    
    /// Helper method to copy directory recursively
    fn copy_directory_recursive<F>(
        &self,
        source_dir: &Path,
        dest_dir: &Path,
        files_processed: &mut usize,
        total_files: usize,
        bytes_processed: &mut u64,
        progress_callback: &Option<F>,
        exclude_patterns: &[String],
    ) -> Result<(), String>
    where
        F: Fn(crate::backup::BackupProgress) + Clone,
    {
        fs::create_dir_all(dest_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        
        let entries = fs::read_dir(source_dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let source_path = entry.path();
            let dest_path = dest_dir.join(entry.file_name());
            
            if should_exclude_path(&source_path, exclude_patterns) {
                continue;
            }
            
            if source_path.is_file() {
                fs::copy(&source_path, &dest_path)
                    .map_err(|e| format!("Failed to copy file: {}", e))?;
                
                let file_size = fs::metadata(&dest_path)
                    .map_err(|e| format!("Failed to get file size: {}", e))?
                    .len();
                
                *files_processed += 1;
                *bytes_processed += file_size;
                
                if let Some(callback) = progress_callback {
                    callback(crate::backup::BackupProgress {
                        current_file: entry.file_name().to_string_lossy().to_string(),
                        files_processed: *files_processed,
                        total_files,
                        bytes_processed: *bytes_processed,
                        total_bytes: 0,
                        current_operation: "Copying files".to_string(),
                    });
                }
            } else if source_path.is_dir() {
                self.copy_directory_recursive(
                    &source_path,
                    &dest_path,
                    files_processed,
                    total_files,
                    bytes_processed,
                    progress_callback,
                    exclude_patterns,
                )?;
            }
        }
        
        Ok(())
    }
}

// Helper functions

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

fn get_folder_priority(name: &str) -> u8 {
    match name.to_lowercase().as_str() {
        "mods" => 1,
        "config" => 2,
        "saves" => 3,
        "resourcepacks" => 4,
        "shaderpacks" => 5,
        "wynntils" => 6,
        "screenshots" => 7,
        "logs" => 8,
        "crash-reports" => 9,
        _ => 10,
    }
}

fn should_skip_item(name: &str) -> bool {
    let skip_items = [
        "manifest.json", "launcher_profiles.json",
        "usernamecache.json", "usercache.json",
        "backup.zip", "tmp_include.zip",
        ".DS_Store", "Thumbs.db", "desktop.ini",
    ];
    
    skip_items.contains(&name) || name.starts_with("tmp_") || name.ends_with(".tmp")
}

fn is_allowed_hidden_item(name: &str) -> bool {
    matches!(name, ".minecraft" | ".fabric" | ".quilt")
}

fn should_exclude_path(path: &Path, exclude_patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    
    for pattern in exclude_patterns {
        if matches_pattern(&path_str, pattern) || matches_pattern(&file_name, pattern) {
            return true;
        }
    }
    
    false
}

fn matches_pattern(text: &str, pattern: &str) -> bool {
    if pattern.contains('*') {
        // Simple glob matching
        let pattern_parts: Vec<&str> = pattern.split('*').collect();
        if pattern_parts.len() == 2 {
            let prefix = pattern_parts[0];
            let suffix = pattern_parts[1];
            text.starts_with(prefix) && text.ends_with(suffix)
        } else {
            // More complex patterns - just check if any part matches
            pattern_parts.iter().any(|part| !part.is_empty() && text.contains(part))
        }
    } else {
        text == pattern || text.ends_with(pattern)
    }
}

pub fn calculate_directory_size(path: &Path) -> Result<u64, String> {
    let mut total_size = 0;
    
    if path.is_file() {
        return Ok(path.metadata()
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .len());
    }
    
    if path.is_dir() {
        let entries = fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let entry_path = entry.path();
            
            if entry_path.is_file() {
                total_size += entry.metadata()
                    .map_err(|e| format!("Failed to get entry metadata: {}", e))?
                    .len();
            } else if entry_path.is_dir() {
                total_size += calculate_directory_size(&entry_path)?;
            }
        }
    }
    
    Ok(total_size)
}

pub fn count_files_recursive(path: &Path) -> Result<usize, String> {
    let mut count = 0;
    
    if path.is_file() {
        return Ok(1);
    }
    
    if path.is_dir() {
        let entries = fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
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
