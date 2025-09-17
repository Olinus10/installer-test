use dioxus::prelude::*;
use crate::installation::Installation;
use crate::backup::{BackupConfig, BackupType, BackupMetadata, BackupProgress, BackupItem, format_bytes};
use log::{debug, error, info};
use std::path::PathBuf;
use dioxus::prelude::MouseEvent;

#[component]
pub fn EnhancedBackupTab(
    installation: Installation,
    installation_id: String,
    onupdate: EventHandler<Installation>,
) -> Element {
    // State for backup operations
    let mut backup_config = use_signal(|| BackupConfig::default());
    let mut is_creating_backup = use_signal(|| false);
    let mut backup_progress = use_signal(|| None::<BackupProgress>);
    let mut operation_error = use_signal(|| Option::<String>::None);
    let mut operation_success = use_signal(|| Option::<String>::None);
    let mut backup_description = use_signal(|| String::new());
    
    // State for backup list
    let mut available_backups = use_signal(|| Vec::<BackupMetadata>::new());
    let mut selected_backup = use_signal(|| Option::<String>::None);
    
    // State for dialogs
    let mut show_backup_config = use_signal(|| false);
    let mut show_restore_confirm = use_signal(|| false);
    let mut show_delete_confirm = use_signal(|| false);
    let mut backup_to_delete = use_signal(|| Option::<String>::None);
    let mut deleting_backup = use_signal(|| false);
    
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

    // Create backup handler for full installation backup
    let create_full_backup = {
        let installation_clone = installation.clone();
        let mut is_creating_backup = is_creating_backup.clone();
        let mut backup_progress = backup_progress.clone();
        let mut operation_error = operation_error.clone();
        let mut operation_success = operation_success.clone();
        let backup_description = backup_description.clone();
        let mut available_backups = available_backups.clone();
        
        move |_| {
            let installation = installation_clone.clone();
            let description = backup_description.read().clone();
            let description = if description.trim().is_empty() {
                format!("Full backup - {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"))
            } else {
                description
            };
            
            is_creating_backup.set(true);
            backup_progress.set(None);
            operation_error.set(None);
            operation_success.set(None);
            
            spawn(async move {
                // Create a full backup configuration
                let full_config = BackupConfig {
                    selected_items: vec!["*".to_string()], // Special marker for full backup
                    compress_backups: true,
                    max_backups: 10,
                    include_hidden_files: true,
                    exclude_patterns: vec![
                        "backups".to_string(),  // Don't backup the backups folder itself
                        "*.log".to_string(),
                        "logs".to_string(),
                        "crash-reports".to_string(),
                        "*.tmp".to_string(),
                    ],
                };
                
                let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel::<BackupProgress>();
                
                let progress_callback = move |progress: BackupProgress| {
                    let _ = progress_tx.send(progress);
                };
                
                let mut backup_progress_clone = backup_progress.clone();
                spawn(async move {
                    while let Some(progress) = progress_rx.recv().await {
                        backup_progress_clone.set(Some(progress));
                    }
                });
                
                match installation.create_backup(
                    BackupType::Manual,
                    &full_config,
                    description.clone(),
                    Some(progress_callback),
                ).await {
                    Ok(metadata) => {
                        operation_success.set(Some(format!("Full backup created successfully: {}", metadata.id)));
                        
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

    // Create selective backup handler
    let create_selective_backup = {
        let installation_clone = installation.clone();
        let mut is_creating_backup = is_creating_backup.clone();
        let mut backup_progress = backup_progress.clone();
        let mut operation_error = operation_error.clone();
        let mut operation_success = operation_success.clone();
        let backup_config = backup_config.clone();
        let backup_description = backup_description.clone();
        let mut available_backups = available_backups.clone();
        let mut show_backup_config = show_backup_config.clone();
        
        move |_| {
            let installation = installation_clone.clone();
            let config = backup_config.read().clone();
            let description = backup_description.read().clone();
            let description = if description.trim().is_empty() {
                format!("Selective backup - {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"))
            } else {
                description
            };
            
            is_creating_backup.set(true);
            backup_progress.set(None);
            operation_error.set(None);
            operation_success.set(None);
            show_backup_config.set(false);
            
            spawn(async move {
                match installation.create_backup(
                    BackupType::Manual,
                    &config,
                    description.clone(),
                    None::<fn(BackupProgress)>,
                ).await {
                    Ok(metadata) => {
                        operation_success.set(Some(format!("Backup created successfully: {}", metadata.id)));
                        
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
    
    // Delete backup handler
    let handle_delete_backup = {
        let installation_clone = installation.clone();
        let mut available_backups = available_backups.clone();
        let mut operation_error = operation_error.clone();
        let mut operation_success = operation_success.clone();
        let backup_to_delete = backup_to_delete.clone();
        let mut show_delete_confirm = show_delete_confirm.clone();
        let mut deleting_backup = deleting_backup.clone();
        
        move |_| {
            if let Some(backup_id) = backup_to_delete.read().clone() {
                let installation = installation_clone.clone();
                let mut available_backups = available_backups.clone();
                let mut operation_error = operation_error.clone();
                let mut operation_success = operation_success.clone();
                let mut deleting_backup = deleting_backup.clone();
                
                deleting_backup.set(true);
                operation_error.set(None);
                
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
                    deleting_backup.set(false);
                });
            }
            show_delete_confirm.set(false);
        }
    };
    
    rsx! {
        div { class: "backup-tab enhanced-backup-tab",
            h2 { "Backup & Restore" }
            p { "Create backups of your installation and restore from previous states. Protect your Wynntils configurations and other important data." }
            
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
                
                div { class: "backup-actions",
                    button {
                        class: "create-backup-button",
                        disabled: *is_creating_backup.read(),
                        onclick: create_full_backup,
                        if *is_creating_backup.read() {
                            "Creating Full Backup..."
                        } else {
                            "📦 Full Backup"
                        }
                    }
                    
                    button {
                        class: "configure-backup-button",
                        onclick: move |_| show_backup_config.set(true),
                        "⚙️ Selective Backup"
                    }
                }
                
                div { class: "backup-info",
                    p { class: "backup-help-text",
                        "💡 Use Full Backup to protect everything including Wynntils configs. Use Selective Backup to choose specific folders."
                    }
                }
                
                // Progress display
                if let Some(progress) = &*backup_progress.read() {
                    EnhancedBackupProgressDisplay { progress: progress.clone() }
                }
            }
            
            // Available Backups Section
            div { class: "backup-section available-backups",
                div { class: "backups-header",
                    h3 { "Available Backups ({available_backups.read().len()})" }
                    
                    if !available_backups.read().is_empty() {
                        div { class: "backup-tools",
                            button {
                                class: "backup-tool-button",
                                onclick: {
                                    let installation_clone = installation.clone();
                                    let mut available_backups = available_backups.clone();
                                    move |_| {
                                        if let Ok(backups) = installation_clone.list_available_backups() {
                                            available_backups.set(backups);
                                        }
                                    }
                                },
                                "🔄 Refresh"
                            }
                        }
                    }
                }
                
                if available_backups.read().is_empty() {
                    div { class: "no-backups",
                        div { class: "no-backups-icon", "📦" }
                        h4 { "No backups available" }
                        p { "Create your first backup above to protect your installation and Wynntils configuration." }
                    }
                } else {
                    div { class: "backups-grid",
                        for backup in available_backups.read().iter() {
                            {
                                let backup_id = backup.id.clone();
                                let is_selected = selected_backup.read().as_ref() == Some(&backup_id);
                                
                                rsx! {
                                    EnhancedBackupCardWithDelete {
                                        backup: backup.clone(),
                                        is_selected: is_selected,
                                        onselect: move |id: String| {
                                            if selected_backup.read().as_ref() == Some(&id) {
                                                selected_backup.set(None);
                                            } else {
                                                selected_backup.set(Some(id));
                                            }
                                        },
                                        ondelete: move |id: String| {
                                            backup_to_delete.set(Some(id));
                                            show_delete_confirm.set(true);
                                        },
                                        onrestore: move |id: String| {
                                            selected_backup.set(Some(id));
                                            show_restore_confirm.set(true);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Simplified Backup Configuration Dialog
if *show_backup_config.read() {
    SimplifiedBackupDialog {
        config: backup_config,
        installation: installation.clone(), // Add this line
        onclose: move |_| show_backup_config.set(false),
        onupdate: move |new_config: BackupConfig| {
            backup_config.set(new_config);
        },
        oncreate: create_selective_backup.clone()
    }
}
            
            // Restore Confirmation Dialog
            if *show_restore_confirm.read() {
                RestoreConfirmationDialog {
                    backup_id: selected_backup.read().clone().unwrap_or_default(),
                    backups: available_backups.read().clone(),
                    installation: installation.clone(),
                    onconfirm: move |_| {
                        // Add restore logic here
                        show_restore_confirm.set(false);
                    },
                    oncancel: move |_| {
                        show_restore_confirm.set(false);
                        selected_backup.set(None);
                    }
                }
            }
            
            // Delete Confirmation Dialog
            if *show_delete_confirm.read() {
                DeleteBackupConfirmationDialog {
                    backup_id: backup_to_delete.read().clone().unwrap_or_default(),
                    backups: available_backups.read().clone(),
                    is_deleting: *deleting_backup.read(),
                    onconfirm: handle_delete_backup,
                    oncancel: move |_| {
                        show_delete_confirm.set(false);
                        backup_to_delete.set(None);
                    }
                }
            }
        }
    }
}

// Simplified backup configuration dialog focusing on key folders
#[component]
fn SimplifiedBackupDialog(
    config: Signal<BackupConfig>,
    installation: Installation, // Add installation parameter
    onclose: EventHandler<()>,
    onupdate: EventHandler<BackupConfig>,
    oncreate: EventHandler<()>,
) -> Element {
    let mut local_config = use_signal(|| config.read().clone());
    let mut discovered_folders = use_signal(|| Vec::<(String, String, bool, bool)>::new()); // (folder_name, description, is_important, default_selected)
    let mut scanning = use_signal(|| true);
    let mut scan_error = use_signal(|| Option::<String>::None);
    
    // Scan installation directory when component loads
    use_effect({
        let installation_path = installation.installation_path.clone();
        let mut discovered_folders = discovered_folders.clone();
        let mut scanning = scanning.clone();
        let mut scan_error = scan_error.clone();
        let mut local_config = local_config.clone();
        
        move || {
            spawn(async move {
                match scan_installation_folders(&installation_path).await {
                    Ok(folders) => {
                        debug!("Discovered {} folders in installation", folders.len());
                        discovered_folders.set(folders.clone());
                        
                        // Auto-select important folders
                        local_config.with_mut(|c| {
                            c.selected_items.clear();
                            for (folder_name, _, is_important, default_selected) in &folders {
                                if *is_important || *default_selected {
                                    c.selected_items.push(folder_name.clone());
                                }
                            }
                        });
                        scanning.set(false);
                    },
                    Err(e) => {
                        error!("Failed to scan installation folders: {}", e);
                        scan_error.set(Some(e));
                        scanning.set(false);
                    }
                }
            });
        }
    });
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container backup-config-dialog enhanced",
                div { class: "modal-header",
                    h3 { "Configure Backup" }
                    button { 
                        class: "modal-close",
                        onclick: move |_| onclose.call(()),
                        "×"
                    }
                }
                
                div { class: "modal-content",
                    div { class: "backup-explanation",
                        p { "Select which folders to include in your backup. Important folders like wynntils and config are selected by default." }
                    }
                    
                    if *scanning.read() {
                        div { class: "scanning-indicator",
                            div { class: "loading-spinner" }
                            span { "Scanning installation folders..." }
                        }
                    } else if let Some(error) = scan_error.read().as_ref() {
                        div { class: "scan-error",
                            "⚠️ Failed to scan folders: {error}"
                        }
                    } else {
                        div { class: "folders-section",
                            h4 { "Installation Folders ({discovered_folders.read().len()} found)" }
                            
                            // Quick select buttons
                            div { class: "quick-select-buttons",
                                button {
                                    class: "quick-select-btn",
                                    r#type: "button",
                                    onclick: move |_| {
                                        local_config.with_mut(|c| {
                                            c.selected_items.clear();
                                            for (folder_name, _, _, _) in discovered_folders.read().iter() {
                                                c.selected_items.push(folder_name.clone());
                                            }
                                        });
                                    },
                                    "Select All"
                                }
                                
                                button {
                                    class: "quick-select-btn",
                                    r#type: "button", 
                                    onclick: move |_| {
                                        local_config.with_mut(|c| {
                                            c.selected_items.clear();
                                            for (folder_name, _, is_important, default_selected) in discovered_folders.read().iter() {
                                                if *is_important || *default_selected {
                                                    c.selected_items.push(folder_name.clone());
                                                }
                                            }
                                        });
                                    },
                                    "Select Important"
                                }
                                
                                button {
                                    class: "quick-select-btn",
                                    r#type: "button",
                                    onclick: move |_| {
                                        local_config.with_mut(|c| c.selected_items.clear());
                                    },
                                    "Clear All"
                                }
                            }
                            
                            // Folder grid
                            div { class: "folders-grid",
                                for (folder_name, description, is_important, _default_selected) in discovered_folders.read().iter() {
                                    {
                                        let folder_path = folder_name.clone();
                                        let is_selected = local_config.read().selected_items.contains(&folder_path);
                                        let folder_class = if *is_important {
                                            "folder-option important"
                                        } else if folder_name == ".bobby" || folder_name == "Distant_Horizons_server_data" {
                                            "folder-option world-data"
                                        } else {
                                            "folder-option"
                                        };
                                        
                                        rsx! {
                                            label { 
                                                class: "{folder_class}",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: is_selected,
                                                    onchange: move |evt| {
                                                        let checked = evt.value() == "true";
                                                        local_config.with_mut(|c| {
                                                            if checked {
                                                                if !c.selected_items.contains(&folder_path) {
                                                                    c.selected_items.push(folder_path.clone());
                                                                }
                                                            } else {
                                                                c.selected_items.retain(|p| p != &folder_path);
                                                            }
                                                        });
                                                    }
                                                }
                                                
                                                div { class: "folder-content",
                                                    div { class: "folder-header",
                                                        span { class: "folder-name", "{folder_name}" }
                                                        if *is_important {
                                                            span { class: "important-badge", "Important" }
                                                        } else if folder_name == ".bobby" || folder_name == "Distant_Horizons_server_data" {
                                                            span { class: "world-data-badge", "World Data" }
                                                        }
                                                    }
                                                    div { class: "folder-description", "{description}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Backup options
                        div { class: "backup-options-section",
                            h4 { "Backup Options" }
                            
                            div { class: "options-grid",
                                label { class: "option-item",
                                    input {
                                        r#type: "checkbox",
                                        checked: local_config.read().compress_backups,
                                        onchange: move |evt| {
                                            local_config.with_mut(|c| c.compress_backups = evt.value() == "true");
                                        }
                                    }
                                    span { "Compress backups (saves space)" }
                                }
                                
                                label { class: "option-item",
                                    input {
                                        r#type: "checkbox",
                                        checked: local_config.read().include_hidden_files,
                                        onchange: move |evt| {
                                            local_config.with_mut(|c| c.include_hidden_files = evt.value() == "true");
                                        }
                                    }
                                    span { "Include hidden files and folders" }
                                }
                                
                                div { class: "option-item number-option",
                                    label { "Maximum backups to keep:" }
                                    input {
                                        r#type: "number",
                                        value: "{local_config.read().max_backups}",
                                        min: "1",
                                        max: "50",
                                        onchange: move |evt| {
                                            if let Ok(value) = evt.value().parse::<usize>() {
                                                local_config.with_mut(|c| c.max_backups = value);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                div { class: "modal-footer",
                    button { 
                        class: "cancel-button",
                        onclick: move |_| onclose.call(()),
                        "Cancel"
                    }
                    
                    button { 
                        class: "create-backup-button",
                        disabled: *scanning.read() || local_config.read().selected_items.is_empty(),
                        onclick: move |_| {
                            onupdate.call(local_config.read().clone());
                            oncreate.call(());
                        },
                        {
                            if *scanning.read() {
                                "Scanning...".to_string()
                            } else {
                                let count = local_config.read().selected_items.len();
                                if count == 0 {
                                    "Select folders first".to_string()
                                } else {
                                    format!("Create Backup ({} folders)", count)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Add this helper function to scan installation folders
async fn scan_installation_folders(installation_path: &std::path::Path) -> Result<Vec<(String, String, bool, bool)>, String> {
    debug!("Scanning installation directory: {:?}", installation_path);
    
    if !installation_path.exists() {
        return Err("Installation directory does not exist".to_string());
    }
    
    let mut folders = Vec::new();
    
    let entries = std::fs::read_dir(installation_path)
        .map_err(|e| format!("Failed to read installation directory: {}", e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        
        // Only include directories, skip files
        if !path.is_dir() {
            continue;
        }
        
        let name = entry.file_name().to_string_lossy().to_string();
        
        // Skip the backups folder itself to avoid recursive backups
        if name == "backups" {
            continue;
        }
        
        // Skip system/temporary folders
        if should_skip_folder(&name) {
            continue;
        }
        
        let (description, is_important, default_selected) = get_folder_info(&name);
        
        folders.push((name, description, is_important, default_selected));
    }
    
    // Sort by importance first, then alphabetically
    folders.sort_by(|a, b| {
        // Important folders first
        match (a.2, b.2) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.0.cmp(&b.0), // Then alphabetically
        }
    });
    
    debug!("Found {} folders in installation", folders.len());
    Ok(folders)
}

fn should_skip_folder(name: &str) -> bool {
    // Skip system files, temp files, and certain game files that shouldn't be backed up
    let skip_patterns = [
        "natives",           // Native libraries
        "versions",          // Minecraft versions (usually symlinked)
        "libraries",         // Minecraft libraries (usually symlinked)
        "assets",           // Minecraft assets (usually symlinked)
        "crash-reports",    // Crash reports (usually don't need backup)
        "logs",             // Log files (usually don't need backup)
        "temp",             // Temporary files
        "cache",            // Cache directories
    ];
    
    skip_patterns.iter().any(|pattern| name == *pattern) || 
    name.starts_with("tmp_") || 
    name.ends_with(".tmp")
}

fn get_folder_info(name: &str) -> (String, bool, bool) {
    match name {
        // Critical/Important folders (always default selected)
        "wynntils" => (
            "Wynntils mod configuration and data - CRITICAL for settings".to_string(),
            true, true
        ),
        "config" => (
            "Game and mod configuration files - Important for settings".to_string(),
            true, true
        ),
        "mods" => (
            "Mod files - Usually managed by installer but good to backup".to_string(),
            true, true
        ),
        
        // World data folders (default selected due to importance)
        ".bobby" => (
            "Bobby mod world cache data - Contains downloaded world chunks".to_string(),
            false, true
        ),
        "Distant_Horizons_server_data" => (
            "Distant Horizons world data - Contains LOD world information".to_string(),
            false, true
        ),
        
        // Resource folders (default selected)
        "resourcepacks" => (
            "Resource pack files - Visual modifications".to_string(),
            false, true
        ),
        "shaderpacks" => (
            "Shader pack files - Visual effects and lighting".to_string(),
            false, true
        ),
        
        // Folders that start with dots (usually mod data)
        name if name.starts_with('.') => {
            if name == ".fabric" || name == ".quilt" {
                ("Mod loader cache - Can be regenerated".to_string(), false, false)
            } else {
                (format!("Mod data folder - {}", name), false, false)
            }
        },
        
        // Generic handling for unknown folders
        _ => {
            // Check if it looks like a mod folder
            if name.contains("_") || name.contains("-") {
                (format!("Mod data folder - {}", name), false, false)
            } else {
                (format!("Custom folder - {}", name), false, false)
            }
        }
    }
}

// Keep existing components for progress display, cards, etc.
#[component]
fn EnhancedBackupProgressDisplay(progress: BackupProgress) -> Element {
    let percentage = if progress.total_files > 0 {
        (progress.files_processed as f64 / progress.total_files as f64 * 100.0) as u32
    } else {
        0
    };
    
    rsx! {
        div { class: "backup-progress enhanced-progress",
            div { class: "progress-header",
                span { class: "operation-status", "{progress.current_operation}" }
                span { class: "progress-percentage", "{percentage}%" }
            }
            
            div { class: "progress-bar-container",
                div { 
                    class: "progress-bar",
                    style: "width: {percentage}%"
                }
            }
            
            div { class: "progress-details",
                div { class: "current-file", "Current: {progress.current_file}" }
                div { class: "file-progress", "Files: {progress.files_processed}/{progress.total_files}" }
                if progress.total_bytes > 0 {
                    div { class: "size-progress", 
                        "Size: {format_bytes(progress.bytes_processed)}/{format_bytes(progress.total_bytes)}" 
                    }
                }
            }
        }
    }
}

#[component]
fn EnhancedBackupCardWithDelete(
    backup: BackupMetadata,
    is_selected: bool,
    onselect: EventHandler<String>,
    ondelete: EventHandler<String>,
    onrestore: EventHandler<String>,
) -> Element {
    let backup_id = backup.id.clone();
    let delete_id = backup.id.clone();
    let restore_id = backup.id.clone();
    
    let mut show_actions = use_signal(|| false);
    
    // Determine backup type display
    let backup_type_display = match backup.backup_type {
        BackupType::Manual => "Manual",
        BackupType::PreUpdate => "Pre-Update",
        BackupType::PreInstall => "Pre-Install",
        BackupType::Scheduled => "Scheduled",
    };
    
    // Check if this is a full backup
    let is_full_backup = backup.included_items.contains(&"*".to_string()) || 
                        backup.included_items.len() > 5;
    
    rsx! {
        div { 
            class: if is_selected {
                "backup-card enhanced selected"
            } else {
                "backup-card enhanced"
            },
            onmouseenter: move |_| show_actions.set(true),
            onmouseleave: move |_| show_actions.set(false),
            onclick: move |_| onselect.call(backup_id.clone()),
            
            div { class: "backup-card-main",
                div { class: "backup-card-header",
                    div { class: "backup-info",
                        h4 { class: "backup-title", "{backup.description}" }
                        
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
                                "{backup_type_display}"
                            }
                            
                            if is_full_backup {
                                span { class: "backup-scope-badge full", "Full" }
                            } else {
                                span { class: "backup-scope-badge selective", "Selective" }
                            }
                        }
                    }
                    
                    span { class: "backup-date", "{backup.age_description()}" }
                }
                
                div { class: "backup-card-details",
                    div { class: "backup-stats-grid",
                        div { class: "backup-stat",
                            span { class: "stat-label", "Size" }
                            span { class: "stat-value", "{backup.formatted_size()}" }
                        }
                        
                        div { class: "backup-stat",
                            span { class: "stat-label", "Files" }
                            span { class: "stat-value", "{backup.file_count}" }
                        }
                        
                        div { class: "backup-stat",
                            span { class: "stat-label", "Items" }
                            span { class: "stat-value", "{backup.included_items.len()}" }
                        }
                        
                        div { class: "backup-stat",
                            span { class: "stat-label", "Version" }
                            span { class: "stat-value", "{backup.modpack_version}" }
                        }
                    }
                    
                    // Show included items preview for selective backups
                    if !is_full_backup && !backup.included_items.is_empty() {
                        div { class: "backup-items-preview",
                            span { class: "items-label", "Includes:" }
                            div { class: "items-tags",
                                for item in backup.included_items.iter().take(4) {
                                    span { 
                                        class: if item == "wynntils" { "item-tag wynntils" } else { "item-tag" },
                                        {item.split('/').last().unwrap_or(item)}
                                    }
                                }
                                if backup.included_items.len() > 4 {
                                    span { class: "item-tag more", 
                                        "+{backup.included_items.len() - 4}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Action buttons (shown on hover or when selected)
            div { 
                class: if *show_actions.read() || is_selected {
                    "backup-card-actions visible"
                } else {
                    "backup-card-actions"
                },
                
                button {
                    class: "backup-action-button restore-button",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        onrestore.call(restore_id.clone());
                    },
                    title: "Restore this backup",
                    "🔄 Restore"
                }
                
                button {
                    class: "backup-action-button delete-button",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        ondelete.call(delete_id.clone());
                    },
                    title: "Delete this backup",
                    "🗑️ Delete"
                }
            }
        }
    }
}

// Keep the existing RestoreConfirmationDialog and DeleteBackupConfirmationDialog components
#[component]
fn RestoreConfirmationDialog(
    backup_id: String,
    backups: Vec<BackupMetadata>,
    installation: Installation,
    onconfirm: EventHandler<()>,
    oncancel: EventHandler<()>,
) -> Element {
    let backup = backups.iter().find(|b| b.id == backup_id);
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container restore-confirm-dialog",
                div { class: "modal-header",
                    h3 { "🔄 Restore Backup" }
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
                        div { class: "backup-details-card",
                            div { class: "detail-row",
                                strong { "Description: " }
                                span { "{backup.description}" }
                            }
                            
                            div { class: "detail-row",
                                strong { "Created: " }
                                span { "{backup.age_description()}" }
                            }
                            
                            div { class: "detail-row",
                                strong { "Size: " }
                                span { "{backup.formatted_size()}" }
                            }
                            
                            div { class: "detail-row",
                                strong { "Files: " }
                                span { "{backup.file_count} files" }
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
                        onclick: move |_| onconfirm.call(()),
                        "Restore Backup"
                    }
                }
            }
        }
    }
}

#[component]
fn DeleteBackupConfirmationDialog(
    backup_id: String,
    backups: Vec<BackupMetadata>,
    is_deleting: bool,
    onconfirm: EventHandler<()>,
    oncancel: EventHandler<()>,
) -> Element {
    let backup = backups.iter().find(|b| b.id == backup_id);
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container delete-backup-dialog",
                div { class: "modal-header danger",
                    h3 { "🗑️ Delete Backup" }
                    if !is_deleting {
                        button { 
                            class: "modal-close",
                            onclick: move |_| oncancel.call(()),
                            "×"
                        }
                    }
                }
                
                div { class: "modal-content",
                    div { class: "danger-warning",
                        "⚠️ This action cannot be undone!"
                    }
                    
                    p { class: "delete-confirmation-text",
                        "Are you sure you want to permanently delete this backup?"
                    }
                    
                    if let Some(backup) = backup {
                        div { class: "backup-details-card",
                            div { class: "detail-row",
                                strong { "Description: " }
                                span { "{backup.description}" }
                            }
                            
                            div { class: "detail-row",
                                strong { "Created: " }
                                span { "{backup.age_description()}" }
                            }
                            
                            div { class: "detail-row",
                                strong { "Size: " }
                                span { "{backup.formatted_size()}" }
                            }
                            
                            div { class: "detail-row",
                                strong { "Files: " }
                                span { "{backup.file_count} files in {backup.included_items.len()} items" }
                            }
                            
                            div { class: "detail-row",
                                strong { "Type: " }
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
                    }
                    
                    if is_deleting {
                        div { class: "deleting-progress",
                            div { class: "loading-spinner" }
                            span { "Deleting backup..." }
                        }
                    }
                }
                
                div { class: "modal-footer",
                    button { 
                        class: "cancel-button",
                        disabled: is_deleting,
                        onclick: move |_| oncancel.call(()),
                        "Cancel"
                    }
                    
                    button { 
                        class: "delete-confirm-button danger",
                        disabled: is_deleting,
                        onclick: move |_| onconfirm.call(()),
                        if is_deleting {
                            "Deleting..."
                        } else {
                            "Delete Backup"
                        }
                    }
                }
            }
        }
    }
}
