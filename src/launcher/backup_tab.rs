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
        onclick: {
            let installation_clone = installation.clone(); // Clone before move
            let mut available_backups = available_backups.clone();
            move |_| {
                // Use the cloned installation
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
    FileSystemBackupDialog {
        installation: installation.clone(),
        config: backup_config,
        onclose: move |_| show_backup_config.set(false),
        onupdate: move |new_config: BackupConfig| {
            backup_config.set(new_config);
        },
        oncreate: create_backup  // Pass the create_backup handler
    }
}
            
            // Restore Confirmation Dialog
if *show_restore_confirm.read() {
    RestoreConfirmationDialog {
        backup_id: selected_backup.read().clone().unwrap_or_default(),
        backups: available_backups.read().clone(),
        installation: installation.clone(), // Make sure this is properly cloned
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

#[component]
fn FileSystemBackupDialog(
    installation: Installation,
    config: Signal<BackupConfig>,
    onclose: EventHandler<()>,
    onupdate: EventHandler<BackupConfig>,
    oncreate: EventHandler<()>,
) -> Element {
    // State for file system tree
    let mut file_tree = use_signal(|| Vec::<FileSystemItem>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| Option::<String>::None);
    let mut expanded_folders = use_signal(|| std::collections::HashSet::<PathBuf>::new());
    let mut search_filter = use_signal(|| String::new());
    
    // Load file system on mount
    use_effect({
        let installation = installation.clone();
        move || {
            loading.set(true);
            error.set(None);
            
            spawn(async move {
                match FileSystemItem::scan_installation(&installation.installation_path) {
                    Ok(items) => {
                        debug!("Scanned {} items from installation", items.len());
                        file_tree.set(items);
                    },
                    Err(e) => {
                        error!("Failed to scan installation: {}", e);
                        error.set(Some(format!("Failed to scan files: {}", e)));
                    }
                }
                loading.set(false);
            });
        }
    });
    
    // Calculate selected size
    let selected_size = use_memo({
        let file_tree = file_tree.clone();
        move || {
            let mut total = 0u64;
            for item in file_tree.read().iter() {
                total += calculate_selected_size(item);
            }
            total
        }
    });
    
    // Filter items based on search
    let filtered_tree = use_memo({
        let file_tree = file_tree.clone();
        let search_filter = search_filter.clone();
        
        move || {
            let filter = search_filter.read().to_lowercase();
            if filter.is_empty() {
                file_tree.read().clone()
            } else {
                filter_tree(&file_tree.read(), &filter)
            }
        }
    });
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container file-backup-dialog",
                div { class: "modal-header",
                    h3 { "Select Files to Backup" }
                    button { 
                        class: "modal-close",
                        onclick: move |_| onclose.call(()),
                        "×"
                    }
                }
                
                div { class: "modal-content",
                    // Search bar
                    div { class: "file-search-bar",
                        input {
                            r#type: "text",
                            placeholder: "Search files and folders...",
                            value: "{search_filter}",
                            oninput: move |evt| search_filter.set(evt.value().clone()),
                            class: "file-search-input"
                        }
                        
                        div { class: "selection-stats",
                            "Selected: {format_bytes(*selected_size.read())}"
                            if config.read().compress_backups {
                                " (~{format_bytes((*selected_size.read() as f64 * 0.65) as u64)} compressed)"
                            }
                        }
                    }
                    
                    // File tree or loading/error state
                    div { class: "file-tree-container",
                        if *loading.read() {
                            div { class: "loading-state",
                                div { class: "loading-spinner" }
                                "Scanning installation files..."
                            }
                        } else if let Some(err) = error.read().as_ref() {
                            div { class: "error-state",
                                "⚠️ {err}"
                            }
                        } else if filtered_tree.read().is_empty() {
                            div { class: "empty-state",
                                if search_filter.read().is_empty() {
                                    "No files found in installation"
                                } else {
                                    "No files match your search"
                                }
                            }
                        } else {
                            div { class: "file-tree",
                                for item in filtered_tree.read().iter() {
                                    FileTreeNode {
                                        item: item.clone(),
                                        expanded_folders: expanded_folders.clone(),
                                        on_toggle: move |path: PathBuf| {
                                            file_tree.with_mut(|items| {
                                                for item in items {
                                                    item.toggle_selection(&path);
                                                }
                                            });
                                        },
                                        on_expand: move |path: PathBuf| {
                                            expanded_folders.with_mut(|set| {
                                                if set.contains(&path) {
                                                    set.remove(&path);
                                                } else {
                                                    set.insert(path);
                                                }
                                            });
                                        },
                                        depth: 0
                                    }
                                }
                            }
                        }
                    }
                    
                    // Quick select buttons
                    div { class: "quick-select-buttons",
                        button {
                            class: "quick-select-btn",
                            onclick: move |_| {
                                file_tree.with_mut(|items| {
                                    for item in items {
                                        select_by_type(item, "mods");
                                    }
                                });
                            },
                            "Select Mods"
                        }
                        
                        button {
                            class: "quick-select-btn",
                            onclick: move |_| {
                                file_tree.with_mut(|items| {
                                    for item in items {
                                        select_by_type(item, "config");
                                    }
                                });
                            },
                            "Select Config"
                        }
                        
                        button {
                            class: "quick-select-btn",
                            onclick: move |_| {
                                file_tree.with_mut(|items| {
                                    for item in items {
                                        item.set_all_children_selected(true);
                                    }
                                });
                            },
                            "Select All"
                        }
                        
                        button {
                            class: "quick-select-btn",
                            onclick: move |_| {
                                file_tree.with_mut(|items| {
                                    for item in items {
                                        item.set_all_children_selected(false);
                                    }
                                });
                            },
                            "Deselect All"
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
                        disabled: *selected_size.read() == 0,
                        onclick: move |_| {
                            // Collect selected items
                            let mut selected_paths = Vec::new();
                            for item in file_tree.read().iter() {
                                selected_paths.extend(item.get_selected_paths());
                            }
                            
                            // Update config with selected paths
                            config.with_mut(|c| {
                                c.selected_items = selected_paths.iter()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .collect();
                            });
                            
                            onupdate.call(config.read().clone());
                            oncreate.call(());
                        },
                        "Create Backup ({format_bytes(*selected_size.read())})"
                    }
                }
            }
        }
    }
}

#[component]
fn FileTreeNode(
    item: FileSystemItem,
    expanded_folders: Signal<std::collections::HashSet<PathBuf>>,
    on_toggle: EventHandler<PathBuf>,
    on_expand: EventHandler<PathBuf>,
    depth: usize,
) -> Element {
    let is_expanded = expanded_folders.read().contains(&item.path);
    let has_children = item.children.as_ref().map_or(false, |c| !c.is_empty());
    let indent = depth * 20;
    
    // Clone the path for each closure to avoid move issues
    let path_for_expand = item.path.clone();
    let path_for_toggle = item.path.clone();
    
    rsx! {
        div { class: "file-tree-node",
            div { 
                class: if item.is_selected { "node-header selected" } else { "node-header" },
                style: "padding-left: {indent}px;",
                
                // Expand/collapse button for directories
                if item.is_directory && has_children {
                    button {
                        class: "expand-btn",
                        onclick: move |_| on_expand.call(path_for_expand.clone()),
                        if is_expanded { "▼" } else { "▶" }
                    }
                } else if item.is_directory {
                    span { class: "expand-spacer" }
                }
                
                // Checkbox
                input {
                    r#type: "checkbox",
                    checked: item.is_selected,
                    onclick: move |_| on_toggle.call(path_for_toggle.clone()),
                    class: "node-checkbox"
                }
                
                // Icon
                span { class: "node-icon",
                    if item.is_directory { "📁" } else { "📄" }
                }
                
                // Name and info
                span { class: "node-name", "{item.name}" }
                
                // Size/count info
                span { class: "node-info",
                    if item.is_directory {
                        if let Some(count) = item.file_count {
                            " ({count} files, {format_bytes(item.size_bytes)})"
                        } else {
                            " ({format_bytes(item.size_bytes)})"
                        }
                    } else {
                        " ({format_bytes(item.size_bytes)})"
                    }
                }
            }
            
            // Children (if expanded)
            if is_expanded && has_children {
                div { class: "node-children",
                    for child in item.children.as_ref().unwrap() {
                        FileTreeNode {
                            item: child.clone(),
                            expanded_folders: expanded_folders.clone(),
                            on_toggle: on_toggle.clone(),
                            on_expand: on_expand.clone(),
                            depth: depth + 1
                        }
                    }
                }
            }
        }
    }
}

// Helper functions
fn calculate_selected_size(item: &FileSystemItem) -> u64 {
    if item.is_selected {
        item.size_bytes
    } else if let Some(children) = &item.children {
        children.iter().map(calculate_selected_size).sum()
    } else {
        0
    }
}

fn filter_tree(items: &[FileSystemItem], filter: &str) -> Vec<FileSystemItem> {
    items.iter()
        .filter_map(|item| {
            let name_matches = item.name.to_lowercase().contains(filter);
            let children_match = item.children.as_ref()
                .map_or(false, |c| !filter_tree(c, filter).is_empty());
            
            if name_matches || children_match {
                let mut filtered_item = item.clone();
                if let Some(children) = &item.children {
                    filtered_item.children = Some(filter_tree(children, filter));
                }
                Some(filtered_item)
            } else {
                None
            }
        })
        .collect()
}

fn select_by_type(item: &mut FileSystemItem, type_name: &str) {
    if item.name == type_name {
        item.set_all_children_selected(true);
    } else if let Some(children) = &mut item.children {
        for child in children {
            select_by_type(child, type_name);
        }
    }
}
