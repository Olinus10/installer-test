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
