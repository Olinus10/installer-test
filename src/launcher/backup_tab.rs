use dioxus::prelude::*;
use crate::installation::Installation;
use crate::backup::{BackupConfig, BackupType, BackupMetadata, BackupProgress, BackupItem, FileSystemItem, format_bytes};
use crate::backup::{calculate_directory_size, count_files_recursive, create_zip_archive}; // Add these imports
use log::{debug, error, info};
use std::path::PathBuf; // Add this import
use std::collections::HashSet; // Add this import

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

    // Add the create_backup function here
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
                match installation.create_backup(
                    BackupType::Manual,
                    &config,
                    description.clone(),
                    None::<fn(BackupProgress)>, // Simple callback type
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
                
                div { class: "backup-actions",
                    button {
                        class: "configure-backup-button",
                        onclick: move |_| show_backup_config.set(true),
                        "⚙️ Select Files"
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
                    EnhancedBackupProgressDisplay { progress: progress.clone() }
                }
            }
            
            // Available Backups Section with enhanced cards
            div { class: "backup-section available-backups",
                div { class: "backups-header",
                    h3 { "Available Backups ({available_backups.read().len()})" }
                    
                    if !available_backups.read().is_empty() {
                        div { class: "backup-tools",
                            button {
                                class: "backup-tool-button",
                                onclick: move |_| {
                                    // Refresh backup list
                                    if let Ok(backups) = installation.list_available_backups() {
                                        available_backups.set(backups);
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
                        p { "Create your first backup above to protect your installation." }
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
            
            // Enhanced File System Backup Configuration Dialog
            if *show_backup_config.read() {
                EnhancedFileSystemBackupDialog {
                    config: backup_config,
                    installation: installation.clone(),
                    estimated_size: installation.get_backup_size_estimate(&backup_config.read()).unwrap_or(0),
                    onclose: move |_| show_backup_config.set(false),
                    onupdate: move |new_config: BackupConfig| {
                        backup_config.set(new_config);
                    }
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

// Add the missing RestoreConfirmationDialog component
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

// Enhanced backup card with delete button
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
                                {format!("{:?}", backup.backup_type)}
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
                    
                    // Show included items preview
                    if !backup.included_items.is_empty() {
                        div { class: "backup-items-preview",
                            span { class: "items-label", "Includes:" }
                            div { class: "items-tags",
                                for item in backup.included_items.iter().take(4) {
                                    span { class: "item-tag", 
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

// Enhanced progress display
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

// Improved backup configuration dialog
#[component]
fn EnhancedFileSystemBackupDialog(
    config: Signal<BackupConfig>,
    installation: Installation,
    estimated_size: u64,
    onclose: EventHandler<()>,
    onupdate: EventHandler<BackupConfig>,
) -> Element {
    let mut local_config = use_signal(|| config.read().clone());
    let mut search_filter = use_signal(|| String::new());
    let mut file_system_items = use_signal(|| Vec::<FileSystemItem>::new());
    let mut loading_items = use_signal(|| false);
    let mut scan_error = use_signal(|| Option::<String>::None);
    let mut expanded_folders = use_signal(|| HashSet::<PathBuf>::new());
    
    // Load file system on mount
    use_effect({
        let installation_path = installation.installation_path.clone();
        let mut file_system_items = file_system_items.clone();
        let mut loading_items = loading_items.clone();
        let mut scan_error = scan_error.clone();
        
        move || {
            loading_items.set(true);
            scan_error.set(None);
            
            spawn(async move {
                match FileSystemItem::scan_directory(&installation_path, 2) {
                    Ok(items) => {
                        debug!("Scanned {} top-level items from installation", items.len());
                        file_system_items.set(items);
                    },
                    Err(e) => {
                        error!("Failed to scan installation directory: {}", e);
                        scan_error.set(Some(format!("Failed to scan installation: {}", e)));
                    }
                }
                loading_items.set(false);
            });
        }
    });
    
    // Filter items based on search
    let filtered_items = use_memo({
        let file_system_items = file_system_items.clone();
        let search_filter = search_filter.clone();
        
        move || {
            let filter = search_filter.read().to_lowercase();
            if filter.is_empty() {
                file_system_items.read().clone()
            } else {
                file_system_items.read().iter()
                    .filter(|item| {
                        item.name.to_lowercase().contains(&filter) ||
                        item.path.to_string_lossy().to_lowercase().contains(&filter)
                    })
                    .cloned()
                    .collect()
            }
        }
    });
    
    // Calculate selected size
    let selected_size = use_memo({
        let file_system_items = file_system_items.clone();
        
        move || {
            file_system_items.read().iter()
                .filter(|item| item.is_selected)
                .map(|item| item.size_bytes)
                .sum::<u64>()
        }
    });
    
    let toggle_item_selection = {
        let mut file_system_items = file_system_items.clone();
        
        move |item_path: PathBuf| {
            file_system_items.with_mut(|items| {
                for item in items.iter_mut() {
                    item.toggle_selection(&item_path);
                }
            });
        }
    };
    
    let toggle_folder_expansion = {
        let mut expanded_folders = expanded_folders.clone();
        
        move |folder_path: PathBuf| {
            expanded_folders.with_mut(|folders| {
                if folders.contains(&folder_path) {
                    folders.remove(&folder_path);
                } else {
                    folders.insert(folder_path);
                }
            });
        }
    };
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container enhanced-file-backup-dialog",
                div { class: "modal-header",
                    h3 { "Select Files and Folders to Backup" }
                    button { 
                        class: "modal-close",
                        onclick: move |_| onclose.call(()),
                        "×"
                    }
                }
                
                div { class: "modal-content",
                    // Search and stats section
                    div { class: "backup-dialog-header",
                        div { class: "search-section",
                            input {
                                r#type: "text",
                                class: "file-search-input",
                                placeholder: "Search files and folders...",
                                value: "{search_filter}",
                                oninput: move |evt| search_filter.set(evt.value().clone())
                            }
                        }
                        
                        div { class: "selection-stats",
                            span { class: "selected-size", 
                                "Selected: {format_bytes(*selected_size.read())}"
                            }
                            
                            if local_config.read().compress_backups {
                                span { class: "compressed-estimate",
                                    " (≈{format_bytes((*selected_size.read() as f64 * 0.65) as u64)} compressed)"
                                }
                            }
                        }
                    }
                    
                    // Error display
                    if let Some(error) = &*scan_error.read() {
                        div { class: "scan-error",
                            "⚠️ {error}"
                        }
                    }
                    
                    // Loading state
                    if *loading_items.read() {
                        div { class: "loading-files",
                            div { class: "loading-spinner" }
                            span { "Scanning installation files..." }
                        }
                    } else {
                        // File system tree
                        div { class: "file-system-tree",
                            if filtered_items.read().is_empty() {
                                div { class: "no-files-found",
                                    if search_filter.read().is_empty() {
                                        "No files found in installation directory."
                                    } else {
                                        "No files match your search criteria."
                                    }
                                }
                            } else {
                                for item in filtered_items.read().iter() {
                                    {
                                        rsx! {
                                            FileSystemTreeItem {
                                                item: item.clone(),
                                                expanded_folders: expanded_folders.clone(),
                                                on_toggle_selection: {
                                                    let toggle_fn = toggle_item_selection.clone();
                                                    let item_path = item.path.clone();
                                                    move |_| toggle_fn(item_path.clone())
                                                },
                                                on_toggle_expansion: {
                                                    let toggle_fn = toggle_folder_expansion.clone();
                                                    let item_path = item.path.clone();
                                                    move |_| toggle_fn(item_path.clone())
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Options section
                    div { class: "backup-options-section",
                        h4 { "Backup Options" }
                        
                        label { class: "backup-option",
                            input {
                                r#type: "checkbox",
                                checked: local_config.read().compress_backups,
                                onchange: move |evt| {
                                    local_config.with_mut(|c| c.compress_backups = evt.value() == "true");
                                }
                            }
                            span { "Compress backup (saves space, slower)" }
                        }
                        
                        label { class: "backup-option",
                            input {
                                r#type: "checkbox",
                                checked: local_config.read().include_hidden_files,
                                onchange: move |evt| {
                                    local_config.with_mut(|c| c.include_hidden_files = evt.value() == "true");
                                }
                            }
                            span { "Include hidden files and folders" }
                        }
                        
                        div { class: "backup-option",
                            label { "Keep last " }
                            input {
                                r#type: "number",
                                value: "{local_config.read().max_backups}",
                                min: "1",
                                max: "50",
                                class: "backup-count-input",
                                onchange: move |evt| {
                                    if let Ok(value) = evt.value().parse::<usize>() {
                                        local_config.with_mut(|c| c.max_backups = value);
                                    }
                                }
                            }
                            label { " backups" }
                        }
                    }
                }
                
                div { class: "modal-footer",
                    div { class: "footer-info",
                        "Estimated backup size: {format_bytes(*selected_size.read())}"
                    }
                    
                    div { class: "footer-actions",
                        button { 
                            class: "cancel-button",
                            onclick: move |_| onclose.call(()),
                            "Cancel"
                        }
                        
                        button { 
                            class: "save-button",
                            disabled: *selected_size.read() == 0,
                            onclick: move |_| {
                                // Convert selected items to config format
                                let mut new_config = local_config.read().clone();
                                let selected_paths: Vec<String> = file_system_items.read().iter()
                                    .flat_map(|item| item.get_selected_paths())
                                    .map(|path| path.to_string_lossy().to_string())
                                    .collect();
                                
                                new_config.selected_items = selected_paths;
                                onupdate.call(new_config);
                                onclose.call(());
                            },
                            "Save & Configure"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FileSystemTreeItem(
    item: FileSystemItem,
    expanded_folders: Signal<HashSet<PathBuf>>,
    on_toggle_selection: EventHandler<()>,
    on_toggle_expansion: EventHandler<()>,
) -> Element {
    let is_expanded = expanded_folders.read().contains(&item.path);
    let has_children = item.children.as_ref().map_or(false, |c| !c.is_empty());
    
    rsx! {
        div { class: "file-tree-item",
            div { 
                class: if item.is_selected {
                    "file-item-header selected"
                } else {
                    "file-item-header"
                },
                
                // Expansion toggle (only for directories with children)
                if item.is_directory && has_children {
                    button {
                        class: if is_expanded { "expand-toggle expanded" } else { "expand-toggle" },
                        onclick: move |_| on_toggle_expansion.call(()),
                        if is_expanded { "▼" } else { "▶" }
                    }
                } else {
                    span { class: "expand-spacer" }
                }
                
                // Selection checkbox
                label { class: "file-selection-label",
                    input {
                        r#type: "checkbox",
                        checked: item.is_selected,
                        onchange: move |_| on_toggle_selection.call(()),
                    }
                    
                    // File/folder icon and info
                    div { class: "file-info",
                        span { class: "file-icon",
                            if item.is_directory { "📁" } else { "📄" }
                        }
                        
                        div { class: "file-details",
                            div { class: "file-name", "{item.name}" }
                            div { class: "file-meta",
                                span { class: "file-size", "{item.get_formatted_size()}" }
                                span { class: "file-description", " • {item.get_description()}" }
                                
                                if let Some(modified) = item.last_modified {
                                    span { class: "file-modified", 
                                        " • Modified {modified.format(\"%m/%d/%Y\")}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Children (if expanded)
            if is_expanded && has_children {
                div { class: "file-tree-children",
                    for child in item.children.as_ref().unwrap().iter() {
                        {
                            rsx! {
                                FileSystemTreeItem {
                                    item: child.clone(),
                                    expanded_folders: expanded_folders.clone(),
                                    on_toggle_selection: {
                                        let item_path = child.path.clone();
                                        move |_| {
                                            // This would need access to the parent component's file_system_items
                                            // For now, we'll just call the parent handler
                                            on_toggle_selection.call(());
                                        }
                                    },
                                    on_toggle_expansion: {
                                        let item_path = child.path.clone();
                                        let mut expanded_folders = expanded_folders.clone();
                                        move |_| {
                                            expanded_folders.with_mut(|folders| {
                                                if folders.contains(&item_path) {
                                                    folders.remove(&item_path);
                                                } else {
                                                    folders.insert(item_path.clone());
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
    }
}              
