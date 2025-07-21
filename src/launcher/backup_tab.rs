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

/// Configuration for what to include in backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub include_mods: bool,
    pub include_config: bool,
    pub include_wynntils: bool,
    pub include_resourcepacks: bool,
    pub include_shaderpacks: bool,
    pub include_saves: bool,
    pub include_screenshots: bool,
    pub include_logs: bool,
    pub compress_backups: bool,
    pub max_backups: usize,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            include_mods: true,
            include_config: true,
            include_wynntils: true,
            include_resourcepacks: true,
            include_shaderpacks: true,
            include_saves: false, // Large files, off by default
            include_screenshots: false, // Large files, off by default
            include_logs: false, // Not usually needed
            compress_backups: true,
            max_backups: 10,
        }
    }
}

/// Metadata about a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub id: String,
    pub description: String,
    pub backup_type: BackupType,
    pub created_at: DateTime<Utc>,
    pub modpack_version: String,
    pub enabled_features: Vec<String>,
    pub file_count: usize,
    pub size_bytes: u64,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupProgress {
    pub current_file: String,
    pub files_processed: usize,
    pub total_files: usize,
    pub bytes_processed: u64,
    pub total_bytes: u64,
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
        
        // Find the most recent working backup
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
        
        // Sort by date, most recent first
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
        // Create safety backup before rollback
        let safety_config = BackupConfig::default();
        let safety_description = format!("Safety backup before rollback to {}", backup_id);
        
        self.installation.create_backup(
            BackupType::PreUpdate,
            &safety_config,
            safety_description,
            None::<fn(BackupProgress)>,
        ).await?;
        
        // Perform the rollback
        self.installation.restore_from_backup(backup_id).await?;
        
        Ok(())
    }
}

/// Rollback option for the UI
#[derive(Debug, Clone)]
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
    progress_callback: Option<F>,
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
    progress_callback: &Option<F>,
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
            
            zip.start_file(&full_name, zip::write::FileOptions::default()
                .compression_method(CompressionMethod::Deflated))?;
            zip.write_all(&file_data)?;
            
            *files_processed += 1;
            *bytes_processed += file_size;
            
            if let Some(callback) = progress_callback {
                callback(BackupProgress {
                    current_file: full_name,
                    files_processed: *files_processed,
                    total_files,
                    bytes_processed: *bytes_processed,
                    total_bytes: 0, // Will be calculated separately
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

use dioxus::prelude::*;
use crate::installation::Installation;
use crate::backup::{BackupConfig, BackupType, BackupMetadata, BackupProgress, RollbackManager, RollbackOption};
use log::{debug, error, info};

#[component]
pub fn BackupTab(
    installation: Installation,
    installation_id: String,
    onupdate: EventHandler<Installation>,
) -> Element {
    // State for backup operations
    let mut backup_config = use_signal(|| BackupConfig::default());
    let mut is_creating_backup = use_signal(|| false);
    let mut is_restoring = use_signal(|| false);
    let mut backup_progress = use_signal(|| None::<BackupProgress>);
    let mut operation_error = use_signal(|| Option::<String>::None);
    let mut operation_success = use_signal(|| Option::<String>::None);
    let mut backup_description = use_signal(|| String::new());
    
    // State for backup list
    let mut available_backups = use_signal(|| Vec::<BackupMetadata>::new());
    let mut selected_backup = use_signal(|| Option::<String>::None);
    let mut show_rollback_confirm = use_signal(|| false);
    
    // State for backup configuration dialog
    let mut show_backup_config = use_signal(|| false);
    
    // Load available backups on component mount
    use_effect({
        let installation_clone = installation.clone();
        let mut available_backups = available_backups.clone();
        
        move || {
            match installation_clone.list_available_backups() {
                Ok(backups) => {
                    debug!("Loaded {} backups for installation", backups.len());
                    available_backups.set(backups);
                },
                Err(e) => {
                    error!("Failed to load backups: {}", e);
                }
            }
        }
    });
    
    // Calculate estimated backup size
    let estimated_size = use_memo({
        let installation_clone = installation.clone();
        let backup_config = backup_config.clone();
        
        move || {
            installation_clone.get_backup_size_estimate(&backup_config.read())
                .unwrap_or(0)
        }
    });
    
    // Create backup function
    let create_backup = {
        let installation_clone = installation.clone();
        let mut is_creating_backup = is_creating_backup.clone();
        let mut backup_progress = backup_progress.clone();
        let mut operation_error = operation_error.clone();
        let mut operation_success = operation_success.clone();
        let backup_config = backup_config.clone();
        let backup_description = backup_description.clone();
        let mut available_backups = available_backups.clone();
        
        move |_| {
            let installation = installation_clone.clone();
            let config = backup_config.read().clone();
            let description = backup_description.read().clone();
            let description = if description.trim().is_empty() {
                format!("Manual backup - {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"))
            } else {
                description
            };
            
            is_creating_backup.set(true);
            backup_progress.set(None);
            operation_error.set(None);
            operation_success.set(None);
            
            spawn(async move {
                let progress_callback = {
                    let mut backup_progress = backup_progress.clone();
                    move |progress: BackupProgress| {
                        backup_progress.set(Some(progress));
                    }
                };
                
                match installation.create_backup(
                    BackupType::Manual,
                    &config,
                    description.clone(),
                    Some(progress_callback),
                ).await {
                    Ok(metadata) => {
                        operation_success.set(Some(format!("Backup created successfully: {}", metadata.id)));
                        
                        // Reload backup list
                        if let Ok(backups) = installation.list_available_backups() {
                            available_backups.set(backups);
                        }
                    },
                    Err(e) => {
                        operation_error.set(Some(format!("Failed to create backup: {}", e)));
                    }
                }
                
                is_creating_backup.set(false);
                backup_progress.set(None);
            });
        }
    };
    
    // Restore backup function
    let restore_backup = {
        let installation_clone = installation.clone();
        let mut is_restoring = is_restoring.clone();
        let mut operation_error = operation_error.clone();
        let mut operation_success = operation_success.clone();
        let selected_backup = selected_backup.clone();
        let onupdate = onupdate.clone();
        
        move |_| {
            if let Some(backup_id) = selected_backup.read().clone() {
                let mut installation = installation_clone.clone();
                
                is_restoring.set(true);
                operation_error.set(None);
                operation_success.set(None);
                
                spawn(async move {
                    match installation.restore_from_backup(&backup_id).await {
                        Ok(_) => {
                            operation_success.set(Some("Installation restored successfully from backup".to_string()));
                            onupdate.call(installation);
                        },
                        Err(e) => {
                            operation_error.set(Some(format!("Failed to restore backup: {}", e)));
                        }
                    }
                    
                    is_restoring.set(false);
                });
            }
        }
    };
    
    // Delete backup function
    let delete_backup = {
        let installation_clone = installation.clone();
        let mut available_backups = available_backups.clone();
        let mut operation_error = operation_error.clone();
        let mut operation_success = operation_success.clone();
        
        move |backup_id: String| {
            let installation = installation_clone.clone();
            
            spawn(async move {
                match installation.delete_backup(&backup_id).await {
                    Ok(_) => {
                        operation_success.set(Some("Backup deleted successfully".to_string()));
                        
                        // Reload backup list
                        if let Ok(backups) = installation.list_available_backups() {
                            available_backups.set(backups);
                        }
                    },
                    Err(e) => {
                        operation_error.set(Some(format!("Failed to delete backup: {}", e)));
                    }
                }
            });
        }
    };
    
    rsx! {
        div { class: "backup-tab",
            h2 { "Backup & Restore" }
            p { "Create backups of your installation and restore from previous states." }
            
            // Display operation messages
            if let Some(error) = &*operation_error.read() {
                div { class: "error-notification backup-error",
                    div { class: "error-message", "{error}" }
                    button { 
                        class: "error-close",
                        onclick: move |_| operation_error.set(None),
                        "×"
                    }
                }
            }
            
            if let Some(success) = &*operation_success.read() {
                div { class: "success-notification",
                    div { class: "success-message", "{success}" }
                    button { 
                        class: "success-close",
                        onclick: move |_| operation_success.set(None),
                        "×"
                    }
                }
            }
            
            // Create Backup Section
            div { class: "backup-section create-backup",
                h3 { "Create New Backup" }
                
                div { class: "backup-description-input",
                    label { r#for: "backup-description", "Description (optional):" }
                    input {
                        id: "backup-description",
                        r#type: "text",
                        value: "{backup_description}",
                        placeholder: "e.g., Before major changes",
                        oninput: move |evt| backup_description.set(evt.value().clone())
                    }
                }
                
                div { class: "backup-size-estimate",
                    "Estimated size: {crate::backup::format_bytes(*estimated_size.read())}"
                }
                
                div { class: "backup-actions",
                    button {
                        class: "configure-backup-button",
                        onclick: move |_| show_backup_config.set(true),
                        "⚙️ Configure"
                    }
                    
                    button {
                        class: "create-backup-button",
                        disabled: *is_creating_backup.read(),
                        onclick: create_backup,
                        if *is_creating_backup.read() {
                            "Creating Backup..."
                        } else {
                            "Create Backup"
                        }
                    }
                }
                
                // Progress display
                if let Some(progress) = &*backup_progress.read() {
                    BackupProgressDisplay { progress: progress.clone() }
                }
            }
            
            // Available Backups Section
            div { class: "backup-section available-backups",
                h3 { "Available Backups ({available_backups.read().len()})" }
                
                if available_backups.read().is_empty() {
                    div { class: "no-backups",
                        "No backups available. Create your first backup above."
                    }
                } else {
                    div { class: "backups-list",
                        for backup in available_backups.read().iter() {
                            {
                                let backup_id = backup.id.clone();
                                let is_selected = selected_backup.read().as_ref() == Some(&backup_id);
                                let delete_backup = delete_backup.clone();
                                
                                rsx! {
                                    BackupCard {
                                        backup: backup.clone(),
                                        is_selected: is_selected,
                                        onselect: move |id: String| {
                                            selected_backup.set(Some(id));
                                        },
                                        ondelete: move |id: String| {
                                            delete_backup(id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Restore actions
                    if selected_backup.read().is_some() {
                        div { class: "restore-actions",
                            button {
                                class: "restore-button",
                                disabled: *is_restoring.read(),
                                onclick: move |_| show_rollback_confirm.set(true),
                                if *is_restoring.read() {
                                    "Restoring..."
                                } else {
                                    "Restore Selected Backup"
                                }
                            }
                        }
                    }
                }
            }
            
            // Backup Configuration Dialog
            if *show_backup_config.read() {
                BackupConfigDialog {
                    config: backup_config,
                    estimated_size: *estimated_size.read(),
                    onclose: move |_| show_backup_config.set(false),
                    onupdate: move |new_config: BackupConfig| {
                        backup_config.set(new_config);
                    }
                }
            }
            
            // Rollback Confirmation Dialog
            if *show_rollback_confirm.read() {
                RollbackConfirmDialog {
                    backup_id: selected_backup.read().clone().unwrap_or_default(),
                    backups: available_backups.read().clone(),
                    onconfirm: restore_backup,
                    oncancel: move |_| show_rollback_confirm.set(false)
                }
            }
        }
    }
}

#[component]
fn BackupCard(
    backup: BackupMetadata,
    is_selected: bool,
    onselect: EventHandler<String>,
    ondelete: EventHandler<String>,
) -> Element {
    let backup_id = backup.id.clone();
    let delete_id = backup.id.clone();
    
    rsx! {
        div { 
            class: if is_selected {
                "backup-card selected"
            } else {
                "backup-card"
            },
            onclick: move |_| onselect.call(backup_id.clone()),
            
            div { class: "backup-card-header",
                div { class: "backup-info",
                    h4 { "{backup.description}" }
                    span { class: "backup-date", "{backup.age_description()}" }
                }
                
                div { class: "backup-badges",
                    span { 
                        class: format!("backup-type-badge {}", 
                            match backup.backup_type {
                                BackupType::Manual => "manual",
                                BackupType::PreUpdate => "pre-update",
                                BackupType::PreInstall => "pre-install",
                                BackupType::Scheduled => "scheduled",
                            }
                        ),
                        {format!("{:?}", backup.backup_type)}
                    }
                }
            }
            
            div { class: "backup-card-details",
                div { class: "backup-detail",
                    span { class: "detail-label", "Version:" }
                    span { class: "detail-value", "{backup.modpack_version}" }
                }
                
                div { class: "backup-detail",
                    span { class: "detail-label", "Size:" }
                    span { class: "detail-value", "{backup.formatted_size()}" }
                }
                
                div { class: "backup-detail",
                    span { class: "detail-label", "Features:" }
                    span { class: "detail-value", "{backup.enabled_features.len()}" }
                }
                
                div { class: "backup-detail",
                    span { class: "detail-label", "Files:" }
                    span { class: "detail-value", "{backup.file_count}" }
                }
            }
            
            div { class: "backup-card-actions",
                button {
                    class: "delete-backup-button",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        ondelete.call(delete_id.clone());
                    },
                    "🗑️"
                }
            }
        }
    }
}

#[component]
fn BackupProgressDisplay(progress: BackupProgress) -> Element {
    let percentage = if progress.total_files > 0 {
        (progress.files_processed as f64 / progress.total_files as f64 * 100.0) as u32
    } else {
        0
    };
    
    rsx! {
        div { class: "backup-progress",
            div { class: "progress-header",
                span { "Creating backup..." }
                span { "{percentage}%" }
            }
            
            div { class: "progress-bar-container",
                div { 
                    class: "progress-bar",
                    style: "width: {percentage}%"
                }
            }
            
            div { class: "progress-details",
                div { "Current: {progress.current_file}" }
                div { "Files: {progress.files_processed}/{progress.total_files}" }
                div { "Size: {crate::backup::format_bytes(progress.bytes_processed)}/{crate::backup::format_bytes(progress.total_bytes)}" }
            }
        }
    }
}

#[component]
fn BackupConfigDialog(
    config: Signal<BackupConfig>,
    estimated_size: u64,
    onclose: EventHandler<()>,
    onupdate: EventHandler<BackupConfig>,
) -> Element {
    let mut local_config = use_signal(|| config.read().clone());
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container backup-config-dialog",
                div { class: "modal-header",
                    h3 { "Backup Configuration" }
                    button { 
                        class: "modal-close",
                        onclick: move |_| onclose.call(()),
                        "×"
                    }
                }
                
                div { class: "modal-content",
                    div { class: "config-section",
                        h4 { "What to include:" }
                        
                        div { class: "config-options",
                            label { class: "config-option",
                                input {
                                    r#type: "checkbox",
                                    checked: local_config.read().include_mods,
                                    onchange: move |evt| {
                                        local_config.with_mut(|c| c.include_mods = evt.value() == "true");
                                    }
                                }
                                "Mods folder"
                            }
                            
                            label { class: "config-option",
                                input {
                                    r#type: "checkbox",
                                    checked: local_config.read().include_config,
                                    onchange: move |evt| {
                                        local_config.with_mut(|c| c.include_config = evt.value() == "true");
                                    }
                                }
                                "Config folder"
                            }
                            
                            label { class: "config-option",
                                input {
                                    r#type: "checkbox",
                                    checked: local_config.read().include_wynntils,
                                    onchange: move |evt| {
                                        local_config.with_mut(|c| c.include_wynntils = evt.value() == "true");
                                    }
                                }
                                "Wynntils folder (settings)"
                            }
                            
                            label { class: "config-option",
                                input {
                                    r#type: "checkbox",
                                    checked: local_config.read().include_resourcepacks,
                                    onchange: move |evt| {
                                        local_config.with_mut(|c| c.include_resourcepacks = evt.value() == "true");
                                    }
                                }
                                "Resource packs"
                            }
                            
                            label { class: "config-option",
                                input {
                                    r#type: "checkbox",
                                    checked: local_config.read().include_shaderpacks,
                                    onchange: move |evt| {
                                        local_config.with_mut(|c| c.include_shaderpacks = evt.value() == "true");
                                    }
                                }
                                "Shader packs"
                            }
                            
                            label { class: "config-option",
                                input {
                                    r#type: "checkbox",
                                    checked: local_config.read().include_saves,
                                    onchange: move |evt| {
                                        local_config.with_mut(|c| c.include_saves = evt.value() == "true");
                                    }
                                }
                                "Saves folder (can be large)"
                            }
                            
                            label { class: "config-option",
                                input {
                                    r#type: "checkbox",
                                    checked: local_config.read().include_screenshots,
                                    onchange: move |evt| {
                                        local_config.with_mut(|c| c.include_screenshots = evt.value() == "true");
                                    }
                                }
                                "Screenshots"
                            }
                            
                            label { class: "config-option",
                                input {
                                    r#type: "checkbox",
                                    checked: local_config.read().include_logs,
                                    onchange: move |evt| {
                                        local_config.with_mut(|c| c.include_logs = evt.value() == "true");
                                    }
                                }
                                "Log files"
                            }
                        }
                    }
                    
                    div { class: "config-section",
                        h4 { "Options:" }
                        
                        label { class: "config-option",
                            input {
                                r#type: "checkbox",
                                checked: local_config.read().compress_backups,
                                onchange: move |evt| {
                                    local_config.with_mut(|c| c.compress_backups = evt.value() == "true");
                                }
                            }
                            "Compress backups (saves space)"
                        }
                        
                        div { class: "config-option",
                            label { "Maximum backups to keep:" }
                            input {
                                r#type: "number",
                                value: "{local_config.read().max_backups}",
                                min: "1",
                                max: "20",
                                onchange: move |evt| {
                                    if let Ok(value) = evt.value().parse::<usize>() {
                                        local_config.with_mut(|c| c.max_backups = value);
                                    }
                                }
                            }
                        }
                    }
                    
                    div { class: "estimated-size",
                        "Estimated backup size: {crate::backup::format_bytes(estimated_size)}"
                    }
                }
                
                div { class: "modal-footer",
                    button { 
                        class: "cancel-button",
                        onclick: move |_| onclose.call(()),
                        "Cancel"
                    }
                    
                    button { 
                        class: "save-button",
                        onclick: move |_| {
                            onupdate.call(local_config.read().clone());
                            onclose.call(());
                        },
                        "Save Configuration"
                    }
                }
            }
        }
    }
}

#[component]
fn RollbackConfirmDialog(
    backup_id: String,
    backups: Vec<BackupMetadata>,
    onconfirm: EventHandler<()>,
    oncancel: EventHandler<()>,
) -> Element {
    let backup = backups.iter().find(|b| b.id == backup_id);
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container rollback-confirm-dialog",
                div { class: "modal-header",
                    h3 { "Confirm Restore" }
                    button { 
                        class: "modal-close",
                        onclick: move |_| oncancel.call(()),
                        "×"
                    }
                }
                
                div { class: "modal-content",
                    div { class: "warning-message",
                        "⚠️ This will replace your current installation with the backup."
                    }
                    
                    if let Some(backup) = backup {
                        div { class: "backup-details",
                            h4 { "Backup Details:" }
                            
                            div { class: "detail-grid",
                                div { class: "detail-row",
                                    span { class: "detail-label", "Description:" }
                                    span { class: "detail-value", "{backup.description}" }
                                }
                                
                                div { class: "detail-row",
                                    span { class: "detail-label", "Created:" }
                                    span { class: "detail-value", "{backup.age_description()}" }
                                }
                                
                                div { class: "detail-row",
                                    span { class: "detail-label", "Version:" }
                                    span { class: "detail-value", "{backup.modpack_version}" }
                                }
                                
                                div { class: "detail-row",
                                    span { class: "detail-label", "Features:" }
                                    span { class: "detail-value", "{backup.enabled_features.len()}" }
                                }
                            }
                        }
                    }
                    
                    div { class: "safety-info",
                        "💡 A safety backup will be created automatically before restoring."
                    }
                }
                
                div { class: "modal-footer",
                    button { 
                        class: "cancel-button",
                        onclick: move |_| oncancel.call(()),
                        "Cancel"
                    }
                    
                    button { 
                        class: "restore-confirm-button",
                        onclick: move |_| {
                            onconfirm.call(());
                            oncancel.call(());
                        },
                        "Restore Backup"
                    }
                }
            }
        }
    }
}

// Enhanced Settings Tab with Backup integration
#[component]
pub fn EnhancedSettingsTab(
    installation: Installation,
    installation_id: String,
    ondelete: EventHandler<()>,
    onupdate: EventHandler<Installation>,
) -> Element {
    let mut active_section = use_signal(|| "general");
    
    rsx! {
        div { class: "enhanced-settings-tab",
            // Section navigation
            div { class: "settings-navigation",
                button {
                    class: if *active_section.read() == "general" { "nav-button active" } else { "nav-button" },
                    onclick: move |_| active_section.set("general"),
                    "General"
                }
                
                button {
                    class: if *active_section.read() == "backup" { "nav-button active" } else { "nav-button" },
                    onclick: move |_| active_section.set("backup"),
                    "Backup & Restore"
                }
                
                button {
                    class: if *active_section.read() == "advanced" { "nav-button active" } else { "nav-button" },
                    onclick: move |_| active_section.set("advanced"),
                    "Advanced"
                }
            }
            
            // Section content
            div { class: "settings-content",
                match *active_section.read() {
                    "general" => rsx! {
                        // Your existing SettingsTab content here
                        crate::launcher::SettingsTab {
                            installation: installation.clone(),
                            installation_id: installation_id.clone(),
                            ondelete: ondelete.clone(),
                            onupdate: onupdate.clone()
                        }
                    },
                    "backup" => rsx! {
                        BackupTab {
                            installation: installation.clone(),
                            installation_id: installation_id.clone(),
                            onupdate: onupdate.clone()
                        }
                    },
                    "advanced" => rsx! {
                        AdvancedSettingsSection {
                            installation: installation.clone(),
                            onupdate: onupdate.clone()
                        }
                    },
                    _ => rsx! { div { "Unknown section" } }
                }
            }
        }
    }
}

#[component]
fn AdvancedSettingsSection(
    installation: Installation,
    onupdate: EventHandler<Installation>,
) -> Element {
    let mut rollback_manager = use_signal(|| None::<RollbackManager>);
    let mut rollback_options = use_signal(|| Vec::<RollbackOption>::new());
    let mut is_loading_options = use_signal(|| false);
    let mut operation_error = use_signal(|| Option::<String>::None);
    
    // Initialize rollback manager
    use_effect({
        let installation_clone = installation.clone();
        let mut rollback_manager = rollback_manager.clone();
        
        move || {
            let manager = RollbackManager::new(installation_clone);
            rollback_manager.set(Some(manager));
        }
    });
    
    // Load rollback options
    let load_rollback_options = {
        let mut rollback_options = rollback_options.clone();
        let mut is_loading_options = is_loading_options.clone();
        let mut operation_error = operation_error.clone();
        let rollback_manager = rollback_manager.clone();
        
        move |_| {
            if let Some(manager) = rollback_manager.read().as_ref() {
                is_loading_options.set(true);
                operation_error.set(None);
                
                match manager.get_rollback_options() {
                    Ok(options) => {
                        rollback_options.set(options);
                    },
                    Err(e) => {
                        operation_error.set(Some(format!("Failed to load rollback options: {}", e)));
                    }
                }
                
                is_loading_options.set(false);
            }
        }
    };
    
    // Rollback to last working state
    let rollback_to_last_working = {
        let rollback_manager = rollback_manager.clone();
        let onupdate = onupdate.clone();
        let mut operation_error = operation_error.clone();
        
        move |_| {
            if let Some(mut manager) = rollback_manager.read().clone() {
                spawn(async move {
                    match manager.rollback_to_last_working().await {
                        Ok(_) => {
                            // Installation was updated, notify parent
                            onupdate.call(manager.installation);
                        },
                        Err(e) => {
                            operation_error.set(Some(format!("Rollback failed: {}", e)));
                        }
                    }
                });
            }
        }
    };
    
    rsx! {
        div { class: "advanced-settings",
            h3 { "Advanced Options" }
            
            if let Some(error) = &*operation_error.read() {
                div { class: "error-notification",
                    div { class: "error-message", "{error}" }
                    button { 
                        class: "error-close",
                        onclick: move |_| operation_error.set(None),
                        "×"
                    }
                }
            }
            
            // Emergency Rollback Section
            div { class: "advanced-section emergency-rollback",
                h4 { "Emergency Recovery" }
                p { "Use these options if your installation is broken or not working properly." }
                
                div { class: "emergency-actions",
                    button {
                        class: "emergency-button rollback-button",
                        onclick: rollback_to_last_working,
                        "🔄 Rollback to Last Working State"
                    }
                    
                    button {
                        class: "emergency-button load-options-button",
                        onclick: load_rollback_options,
                        disabled: *is_loading_options.read(),
                        if *is_loading_options.read() {
                            "Loading..."
                        } else {
                            "📋 Show All Rollback Options"
                        }
                    }
                }
                
                // Display rollback options if loaded
                if !rollback_options.read().is_empty() {
                    div { class: "rollback-options",
                        h5 { "Available Rollback Points:" }
                        
                        for option in rollback_options.read().iter() {
                            {
                                let option_id = option.backup_id.clone();
                                let rollback_manager = rollback_manager.clone();
                                let onupdate = onupdate.clone();
                                
                                rsx! {
                                    RollbackOptionCard {
                                        option: option.clone(),
                                        onrollback: move |backup_id: String| {
                                            if let Some(mut manager) = rollback_manager.read().clone() {
                                                spawn(async move {
                                                    match manager.rollback_to_backup(&backup_id).await {
                                                        Ok(_) => {
                                                            onupdate.call(manager.installation);
                                                        },
                                                        Err(e) => {
                                                            // Handle error
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Installation Health Check
            div { class: "advanced-section health-check",
                h4 { "Installation Health" }
                
                div { class: "health-info",
                    div { class: "health-item",
                        span { class: "health-label", "Installation Status:" }
                        span { 
                            class: if installation.installed {
                                "health-value status-good"
                            } else {
                                "health-value status-bad"
                            },
                            if installation.installed { "✅ Installed" } else { "❌ Not Installed" }
                        }
                    }
                    
                    div { class: "health-item",
                        span { class: "health-label", "Last Used:" }
                        span { class: "health-value", "{installation.last_used.format(\"%Y-%m-%d %H:%M\")}" }
                    }
                    
                    div { class: "health-item",
                        span { class: "health-label", "Total Launches:" }
                        span { class: "health-value", "{installation.total_launches}" }
                    }
                    
                    div { class: "health-item",
                        span { class: "health-label", "Enabled Features:" }
                        span { class: "health-value", "{installation.enabled_features.len()}" }
                    }
                }
                
                button {
                    class: "health-check-button",
                    "🔍 Run Diagnostic Check"
                }
            }
        }
    }
}

#[component]
fn RollbackOptionCard(
    option: RollbackOption,
    onrollback: EventHandler<String>,
) -> Element {
    let backup_id = option.backup_id.clone();
    
    rsx! {
        div { 
            class: if option.is_recommended {
                "rollback-option-card recommended"
            } else {
                "rollback-option-card"
            },
            
            div { class: "option-header",
                h6 { "{option.description}" }
                
                if option.is_recommended {
                    span { class: "recommended-badge", "Recommended" }
                }
            }
            
            div { class: "option-details",
                span { "Version: {option.modpack_version}" }
                span { "Created: {option.created_at.format(\"%Y-%m-%d %H:%M\")}" }
                span { "Size: {crate::backup::format_bytes(option.size)}" }
            }
            
            button {
                class: "rollback-option-button",
                onclick: move |_| onrollback.call(backup_id.clone()),
                "Rollback to This Point"
            }
        }
    }
}
