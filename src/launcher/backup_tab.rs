use dioxus::prelude::*;
use crate::installation::Installation;
use crate::backup::{BackupConfig, BackupType, BackupMetadata, BackupProgress, BackupItem, format_bytes};
use log::{debug, error, info};

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
    let mut show_rollback_confirm = use_signal(|| false);
    let mut backup_to_delete = use_signal(|| Option::<String>::None);
    let mut show_delete_confirm = use_signal(|| false);
    
    // State for configuration dialog
    let mut show_backup_config = use_signal(|| false);
    let mut available_items = use_signal(|| Vec::<BackupItem>::new());
    let mut loading_items = use_signal(|| false);
    
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
    
    // Load available backup items when needed
    let load_backup_items = {
        let installation_clone = installation.clone();
        let mut available_items = available_items.clone();
        let mut loading_items = loading_items.clone();
        let mut operation_error = operation_error.clone();
        
        move |_| {
            let installation = installation_clone.clone();
            let mut available_items = available_items.clone();
            let mut loading_items = loading_items.clone();
            let mut operation_error = operation_error.clone();
            
            loading_items.set(true);
            operation_error.set(None);
            
            spawn(async move {
                match installation.discover_backup_items() {
                    Ok(items) => {
                        debug!("Discovered {} backup items", items.len());
                        available_items.set(items);
                    },
                    Err(e) => {
                        error!("Failed to discover backup items: {}", e);
                        operation_error.set(Some(format!("Failed to scan installation: {}", e)));
                        available_items.set(Vec::new());
                    }
                }
                loading_items.set(false);
            });
        }
    };

    // Calculate estimated backup size
    let estimated_size = use_memo({
        let installation_clone = installation.clone();
        let backup_config = backup_config.clone();
        
        move || {
            let config = backup_config.read();
            installation_clone.get_backup_size_estimate(&config).unwrap_or(0)
        }
    });
    
    // Create backup function with immediate progress display
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
            
            // IMPROVED: Set progress state immediately when clicked
            is_creating_backup.set(true);
            backup_progress.set(Some(BackupProgress {
                current_file: "Preparing backup...".to_string(),
                files_processed: 0,
                total_files: 0,
                bytes_processed: 0,
                total_bytes: 0,
                current_operation: "Starting backup...".to_string(),
            }));
            operation_error.set(None);
            operation_success.set(None);
            
            spawn(async move {
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
                    &config,
                    description.clone(),
                    Some(progress_callback),
                ).await {
                    Ok(metadata) => {
                        operation_success.set(Some(format!("Backup created successfully: {}", metadata.id)));
                        
                        if let Ok(backups) = installation.list_available_backups() {
                            available_backups.set(backups);
                        }
                        
                        // Auto-clear progress after success
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        backup_progress.set(None);
                    },
                    Err(e) => {
                        operation_error.set(Some(format!("Failed to create backup: {}", e)));
                        backup_progress.set(None);
                    }
                }
                
                is_creating_backup.set(false);
            });
        }
    };
    
    // Delete and restore functions (simplified for your existing structures)
    let delete_backup = {
        let installation_clone = installation.clone();
        let mut available_backups = available_backups.clone();
        let mut operation_error = operation_error.clone();
        let mut operation_success = operation_success.clone();
        let mut backup_to_delete = backup_to_delete.clone();
        let mut show_delete_confirm = show_delete_confirm.clone();
        
        move |backup_id: String| {
            backup_to_delete.set(Some(backup_id));
            show_delete_confirm.set(true);
        }
    };
    
    let confirm_delete_backup = {
        let installation_clone = installation.clone();
        let mut available_backups = available_backups.clone();
        let mut operation_error = operation_error.clone();
        let mut operation_success = operation_success.clone();
        let backup_to_delete = backup_to_delete.clone();
        let mut show_delete_confirm = show_delete_confirm.clone();
        
        move |_| {
            if let Some(backup_id) = backup_to_delete.read().clone() {
                let installation = installation_clone.clone();
                let mut available_backups = available_backups.clone();
                let mut operation_error = operation_error.clone();
                let mut operation_success = operation_success.clone();
                
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
                
                div { class: "backup-size-estimate",
                    "Estimated size: {format_bytes(*estimated_size.read())}"
                }
                
                div { class: "backup-actions",
                    button {
                        class: "configure-backup-button",
                        onclick: move |_| {
                            load_backup_items(());
                            show_backup_config.set(true);
                        },
                        "⚙️ Configure Items"
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
                
                // Progress display that shows immediately
                if let Some(progress) = &*backup_progress.read() {
                    EnhancedBackupProgressDisplay { progress: progress.clone() }
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
                                let mut delete_backup_callback = delete_backup.clone();
                                
                                rsx! {
                                    EnhancedBackupCard {
                                        backup: backup.clone(),
                                        is_selected: is_selected,
                                        onselect: move |id: String| {
                                            selected_backup.set(Some(id));
                                        },
                                        ondelete: move |id: String| {
                                            delete_backup_callback(id);
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
                                onclick: move |_| show_rollback_confirm.set(true),
                                "Restore Selected Backup"
                            }
                        }
                    }
                }
            }
            
            // Enhanced Backup Configuration Dialog
            if *show_backup_config.read() {
                ImprovedBackupConfigDialog {
                    config: backup_config,
                    available_items: available_items.read().clone(),
                    loading_items: *loading_items.read(),
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
                    onconfirm: move |_| {
                        // Add restore logic here
                        show_rollback_confirm.set(false);
                    },
                    oncancel: move |_| show_rollback_confirm.set(false)
                }
            }
            
            // Delete Confirmation Dialog
            if *show_delete_confirm.read() {
                DeleteBackupConfirmDialog {
                    backup_id: backup_to_delete.read().clone().unwrap_or_default(),
                    backups: available_backups.read().clone(),
                    onconfirm: confirm_delete_backup,
                    oncancel: move |_| {
                        show_delete_confirm.set(false);
                        backup_to_delete.set(None);
                    }
                }
            }
        }
    }
}

// Enhanced backup card with delete button
#[component]
fn EnhancedBackupCard(
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
                    span { class: "detail-label", "Items:" }
                    span { class: "detail-value", "{backup.included_items.len()}" }
                }
                
                div { class: "backup-detail",
                    span { class: "detail-label", "Files:" }
                    span { class: "detail-value", "{backup.file_count}" }
                }
                
                // Show included items
                if !backup.included_items.is_empty() {
                    div { class: "backup-items-list",
                        span { class: "items-label", "Includes:" }
                        div { class: "items-chips",
                            for item in backup.included_items.iter().take(3) {
                                span { class: "item-chip", "{item}" }
                            }
                            if backup.included_items.len() > 3 {
                                span { class: "item-chip more", "+{backup.included_items.len() - 3} more" }
                            }
                        }
                    }
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
fn ImprovedBackupConfigDialog(
    config: Signal<BackupConfig>,
    available_items: Vec<BackupItem>,
    loading_items: bool,
    estimated_size: u64,
    onclose: EventHandler<()>,
    onupdate: EventHandler<BackupConfig>,
) -> Element {
    let mut local_config = use_signal(|| config.read().clone());
    let mut search_filter = use_signal(|| String::new());
    
    // Filter items based on search
    let filtered_items = use_memo({
        let available_items = available_items.clone();
        let search_filter = search_filter.clone();
        
        move || {
            let filter = search_filter.read().to_lowercase();
            if filter.is_empty() {
                available_items.clone()
            } else {
                available_items.iter()
                    .filter(|item| {
                        item.name.to_lowercase().contains(&filter) ||
                        item.description.as_ref()
                            .map(|d| d.to_lowercase().contains(&filter))
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            }
        }
    });
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container enhanced-backup-config-dialog",
                div { class: "modal-header",
                    h3 { "Configure Backup Items" }
                    button { 
                        class: "modal-close",
                        onclick: move |_| onclose.call(()),
                        "×"
                    }
                }
                
                div { class: "modal-content",
                    // Search and filter section
                    div { class: "config-search-section",
                        input {
                            r#type: "text",
                            class: "search-input",
                            placeholder: "Search items...",
                            value: "{search_filter}",
                            oninput: move |evt| search_filter.set(evt.value().clone())
                        }
                    }
                    
                    // Loading state
                    if loading_items {
                        div { class: "loading-items",
                            div { class: "loading-spinner" }
                            span { "Scanning installation..." }
                        }
                    }
                    
                    // Items selection with actual discovered files
                    if !loading_items && !filtered_items.read().is_empty() {
                        div { class: "config-section items-selection",
                            h4 { "Select items to backup:" }
                            
                            div { class: "items-grid",
                                for item in filtered_items.read().iter() {
                                    {
                                        let item_path = item.path.to_string_lossy().to_string();
                                        let is_selected = local_config.read().selected_items.contains(&item_path);
                                        let size_display = if item.is_directory {
                                            format!("{} ({} files)", 
                                                format_bytes(item.size_bytes),
                                                item.file_count.unwrap_or(0)
                                            )
                                        } else {
                                            format_bytes(item.size_bytes)
                                        };
                                        
                                        rsx! {
                                            label { 
                                                class: if is_selected { 
                                                    "backup-item-option selected" 
                                                } else { 
                                                    "backup-item-option" 
                                                },
                                                
                                                input {
                                                    r#type: "checkbox",
                                                    checked: is_selected,
                                                    onchange: move |evt| {
                                                        let checked = evt.value() == "true";
                                                        local_config.with_mut(|c| {
                                                            if checked {
                                                                if !c.selected_items.contains(&item_path) {
                                                                    c.selected_items.push(item_path.clone());
                                                                }
                                                            } else {
                                                                c.selected_items.retain(|p| p != &item_path);
                                                            }
                                                        });
                                                    }
                                                }
                                                
                                                div { class: "item-info",
                                                    div { class: "item-header",
                                                        span { class: "item-icon", 
                                                            if item.is_directory { "📁" } else { "📄" }
                                                        }
                                                        span { class: "item-name", "{item.name}" }
                                                    }
                                                    
                                                    if let Some(description) = &item.description {
                                                        div { class: "item-description", "{description}" }
                                                    }
                                                    
                                                    div { class: "item-size", "{size_display}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    if !loading_items && filtered_items.read().is_empty() && !available_items.is_empty() {
                        div { class: "no-results",
                            "No items match your search criteria."
                        }
                    }
                    
                    if !loading_items && available_items.is_empty() {
                        div { class: "no-items",
                            "No items found in installation directory."
                        }
                    }
                    
                    // Advanced options
                    div { class: "config-section advanced-options",
                        h4 { "Advanced Options:" }
                        
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
                        
                        label { class: "config-option",
                            input {
                                r#type: "checkbox",
                                checked: local_config.read().include_hidden_files,
                                onchange: move |evt| {
                                    local_config.with_mut(|c| c.include_hidden_files = evt.value() == "true");
                                }
                            }
                            "Include hidden files and folders"
                        }
                        
                        div { class: "config-option",
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
                    
                    div { class: "estimated-size",
                        "Estimated backup size: {format_bytes(estimated_size)}"
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

// Delete confirmation dialog
#[component]
fn DeleteBackupConfirmDialog(
    backup_id: String,
    backups: Vec<BackupMetadata>,
    onconfirm: EventHandler<()>,
    oncancel: EventHandler<()>,
) -> Element {
    let backup = backups.iter().find(|b| b.id == backup_id);
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container delete-backup-confirm-dialog",
                div { class: "modal-header",
                    h3 { "Delete Backup" }
                    button { 
                        class: "modal-close",
                        onclick: move |_| oncancel.call(()),
                        "×"
                    }
                }
                
                div { class: "modal-content",
                    div { class: "warning-message",
                        "⚠️ Are you sure you want to delete this backup? This action cannot be undone."
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
                                    span { class: "detail-label", "Size:" }
                                    span { class: "detail-value", "{backup.formatted_size()}" }
                                }
                                
                                div { class: "detail-row",
                                    span { class: "detail-label", "Files:" }
                                    span { class: "detail-value", "{backup.file_count}" }
                                }
                            }
                        }
                    }
                }
                
                div { class: "modal-footer",
                    button { 
                        class: "cancel-button",
                        onclick: move |_| oncancel.call(()),
                        "Cancel"
                    }
                    
                    button { 
                        class: "delete-confirm-button",
                        onclick: move |_| onconfirm.call(()),
                        "Delete Backup"
                    }
                }
            }
        }
    }
}

// Rollback confirmation dialog
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
                                    span { class: "detail-label", "Items:" }
                                    span { class: "detail-value", "{backup.included_items.len()}" }
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
