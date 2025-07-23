use dioxus::prelude::*;
use crate::installation::{Installation, delete_installation};
use crate::backup::{BackupConfig, BackupType, BackupMetadata, BackupProgress};
use log::{debug, error};
use std::path::PathBuf;

#[component]
pub fn SettingsTab(
    installation: Installation,
    installation_id: String,
    ondelete: EventHandler<()>,
    onupdate: EventHandler<Installation>,
) -> Element {
    // Remove the tab navigation - everything goes in one page now
    rsx! {
        div { class: "settings-tab",
            h2 { "Installation Settings" }
            
            GeneralSettingsSection {
                installation: installation.clone(),
                installation_id: installation_id.clone(),
                ondelete: ondelete.clone(),
                onupdate: onupdate.clone()
            }
        }
    }
}

#[component]
fn GeneralSettingsSection(
    installation: Installation,
    installation_id: String,
    ondelete: EventHandler<()>,
    onupdate: EventHandler<Installation>,
) -> Element {
    // Clone everything we need to use across closures
    let installation_path_display = installation.installation_path.display().to_string();
    let installation_name = installation.name.clone();
    let installation_id_for_delete = installation_id.clone();
    let installation_id_for_cache = installation_id.clone();
    
    // Clone values needed in the UI (outside of closures)
    let path_for_ui = installation.installation_path.clone();
    let minecraft_version = installation.minecraft_version.clone();
    let loader_type = installation.loader_type.clone();
    let loader_version = installation.loader_version.clone();
    let launcher_type = installation.launcher_type.clone();
    let created_at = installation.created_at;
    let last_used = installation.last_used;
    let total_launches = installation.total_launches;
    let last_launch = installation.last_launch;
    
    // State for rename dialog
    let mut show_rename_dialog = use_signal(|| false);
    let mut new_name = use_signal(|| installation_name.clone());
    let mut rename_error = use_signal(|| Option::<String>::None);
    
    // State for delete confirmation
    let mut show_delete_confirm = use_signal(|| false);
    
    // State for operation status
    let mut operation_error = use_signal(|| Option::<String>::None);
    let mut is_operating = use_signal(|| false);
    
    // Backup-related state
    let mut show_backup_section = use_signal(|| false);
    let mut backup_config = use_signal(|| BackupConfig::default());
    let mut backup_description = use_signal(|| String::new());
    let mut available_backups = use_signal(|| Vec::<BackupMetadata>::new());
    let mut selected_backup = use_signal(|| Option::<String>::None);
    let mut is_creating_backup = use_signal(|| false);
    let mut backup_progress = use_signal(|| None::<BackupProgress>);
    let mut backup_success = use_signal(|| Option::<String>::None);
    let mut show_backup_config = use_signal(|| false);
    let mut show_restore_confirm = use_signal(|| false);
    
    // Load available backups when backup section is shown
    use_effect({
        let installation_clone = installation.clone();
        let mut available_backups = available_backups.clone();
        let show_backup_section = show_backup_section.clone();
        
        move || {
            if *show_backup_section.read() {
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
        }
    });
    
    // Open folder function
    let installation_path_for_folder = installation.installation_path.clone();
    let open_folder = move |_| {
        let mut path = installation_path_for_folder.clone();
        
        debug!("Opening installation folder: {:?}", path);
        
        // Launch appropriate command based on OS
        #[cfg(target_os = "windows")]
        let result = {
            let path_str = path.to_string_lossy().replace("/", "\\");
            std::process::Command::new("explorer")
                .arg(&path_str)
                .spawn()
        };
        
        #[cfg(target_os = "macos")]
        let result = std::process::Command::new("open")
            .arg(path)
            .spawn();
            
        #[cfg(target_os = "linux")]
        let result = std::process::Command::new("xdg-open")
            .arg(path)
            .spawn();
            
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        let result = Err(std::io::Error::new(std::io::ErrorKind::Other, "Unsupported platform"));
        
        if let Err(e) = result {
            debug!("Failed to open installation folder: {}", e);
            operation_error.set(Some(format!("Failed to open folder: {}", e)));
        }
    };
    
    // Handle rename
    let installation_for_rename = installation.clone();
    let handle_rename = move |_| {
        let mut installation_copy = installation_for_rename.clone();
        let new_name_trimmed = new_name.read().trim().to_string();
        installation_copy.name = new_name_trimmed.clone();
        
        const MAX_NAME_LENGTH: usize = 25;
        if new_name_trimmed.is_empty() {
            rename_error.set(Some("Installation name cannot be empty".to_string()));
            return;
        }
        
        if new_name_trimmed.len() > MAX_NAME_LENGTH {
            rename_error.set(Some(format!("Installation name cannot exceed {} characters.", MAX_NAME_LENGTH)));
            return;
        }
        
        is_operating.set(true);
        
        match installation_copy.save() {
            Ok(_) => {
                debug!("Renamed installation to: {}", installation_copy.name);
                show_rename_dialog.set(false);
                is_operating.set(false);
                rename_error.set(None);
                onupdate.call(installation_copy);
            },
            Err(e) => {
                debug!("Failed to rename installation: {}", e);
                rename_error.set(Some(format!("Failed to rename installation: {}", e)));
                is_operating.set(false);
            }
        }
    };
    
    // Handle delete
    let handle_delete = move |_| {
        let id_to_delete = installation_id_for_delete.clone();
        let delete_handler = ondelete.clone();
        let mut operation_error_clone = operation_error.clone();
        let mut is_operating_clone = is_operating.clone();
        
        is_operating_clone.set(true);
        
        spawn(async move {
            match crate::installation::delete_installation(&id_to_delete) {
                Ok(_) => {
                    debug!("Successfully deleted installation: {}", id_to_delete);
                    delete_handler.call(());
                },
                Err(e) => {
                    error!("Failed to delete installation: {}", e);
                    operation_error_clone.set(Some(format!("Failed to delete installation: {}", e)));
                    is_operating_clone.set(false);
                }
            }
        });
    };
    
    // Backup functions
    let create_backup = {
        let installation_clone = installation.clone();
        let mut is_creating_backup = is_creating_backup.clone();
        let mut backup_progress = backup_progress.clone();
        let mut operation_error = operation_error.clone();
        let mut backup_success = backup_success.clone();
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
            backup_success.set(None);
            
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
                        backup_success.set(Some(format!("Backup created successfully: {}", metadata.id)));
                        
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
    
rsx! {
    div { class: "settings-tab",
        // Display operation error if any
        {operation_error.read().as_ref().map(|error| rsx! {
            div { class: "error-notification settings-error",
                div { class: "error-message", "{error}" }
                button { 
                    class: "error-close",
                    onclick: move |_| operation_error.set(None),
                    "×"
                }
            }
        })}
        
        // Success message for backups
        {backup_success.read().as_ref().map(|success| rsx! {
            div { class: "success-notification",
                div { class: "success-message", "{success}" }
                button { 
                    class: "success-close",
                    onclick: move |_| backup_success.set(None),
                    "×"
                }
            }
        })}
        
        // Installation information section
        div { class: "settings-section installation-info",
            h3 { "Installation Information" }
            
            div { class: "info-grid",
                div { class: "info-row",
                    div { class: "info-label", "Name: ", "{installation_name}" }
                }
                
                div { class: "info-row",
                    div { class: "info-label", "Created: ", "{created_at.format(\"%B %d, %Y\")}" }
                }
                
                div { class: "info-row",
                    div { class: "info-label", "Last Used: ", "{last_used.format(\"%B %d, %Y\")}"  }
                }
                
                div { class: "info-row",
                    div { class: "info-label", "Minecraft: ", "{minecraft_version}" }
                }
                
                div { class: "info-row",
                    div { class: "info-label", "Loader: ", "{loader_type} {loader_version}"  }
                }
                
                div { class: "info-row",
                    div { class: "info-label", "Launcher: ", "{launcher_type}" }
                }
                
                div { class: "info-row path-row",
                    div { class: "info-label", "Path:" }
                    div { class: "path-container", 
                        code { class: "installation-path-code", "{installation_path_display}" }
                    }
                }
            }
        }
        
        // Usage statistics section
        div { class: "settings-section usage-stats",
            h3 { "Usage Statistics" }
            
            div { class: "stats-grid",
                div { class: "stat-item",
                    div { class: "stat-value", "{total_launches}" }
                    div { class: "stat-label", "Total Launches" }
                }
                
                div { class: "stat-item",
                    div { class: "stat-value",
                        {match last_launch {
                            Some(launch_date) => format!("{}", launch_date.format("%B %d, %Y")),
                            None => "Never".to_string(),
                        }}
                    }
                    div { class: "stat-label", "Last Launch" }
                }
            }
        }
        
        // Advanced section with backup integration
        div { class: "settings-section advanced-settings",
            h3 { "Advanced" }
            
            div { class: "advanced-description",
                p { "These settings allow you to directly manage your installation. Use with caution." }
            }
            
            // Backup & Restore option
            div { class: "advanced-option",
                div { class: "advanced-option-info",
                    h4 { "Backup & Restore" }
                    p { "Create backups of your installation and restore from previous states to protect your configuration and settings." }
                }
                
                button {
                    class: "advanced-button backup-button",
                    disabled: *is_operating.read(),
                    onclick: move |_| {
                        let current_state = *show_backup_section.read();
                        show_backup_section.set(!current_state);
                    },
                    {if *show_backup_section.read() {
                        "Hide Backup Options"
                    } else {
                        "Show Backup Options"
                    }}
                }
            }
            
            // Expandable backup section
          {if *show_backup_section.read() {
    Some(rsx! {
        div { class: "backup-expanded-section",
            // Quick backup creation
            div { class: "backup-quick-create",
                h5 { "Quick Backup" }
                
                div { class: "backup-description-input",
                    input {
                        r#type: "text",
                        value: "{backup_description}",
                        placeholder: "Backup description (optional)",
                        oninput: move |evt| backup_description.set(evt.value().clone())
                    }
                }
                
                div { class: "backup-quick-actions",
                    button {
                        class: "backup-config-button",
                        onclick: move |_| show_backup_config.set(true),
                        "⚙️ Configure"
                    }
                    
                    button {
                        class: "create-backup-button",
                        disabled: *is_creating_backup.read(),
                        onclick: create_backup,
                        {if *is_creating_backup.read() {
                            "Creating..."
                        } else {
                            "Create Backup"
                        }}
                    }
                }
                
                // Progress display - Fixed version
                {backup_progress.read().as_ref().map(|progress| {
                    let progress_text = format!("Creating backup... {}/{} files", progress.files_processed, progress.total_files);
                    let progress_percentage = if progress.total_files > 0 { 
                        (progress.files_processed as f64 / progress.total_files as f64 * 100.0) as u32 
                    } else { 
                        0 
                    };
                    
                    rsx! {
                        div { class: "backup-progress-mini",
                            div { class: "progress-text", "{progress_text}" }
                            div { class: "progress-bar-mini",
                                div { 
                                    class: "progress-fill",
                                    style: "width: {progress_percentage}%"
                                }
                            }
                        }
                    }
                })}
            }
            
            // Available backups list - Fixed version
            div { class: "backup-list-section",
                h5 { "Available Backups ({available_backups.read().len()})" }
                
                {if available_backups.read().is_empty() {
                    rsx! {
                        div { class: "no-backups-mini",
                            "No backups available. Create your first backup above."
                        }
                    }
                } else {
                    // Create backup items outside of rsx! macro
                    let backup_items: Vec<_> = available_backups.read().iter().take(3).map(|backup| {
                        let backup_id = backup.id.clone();
                        let is_selected = selected_backup.read().as_ref() == Some(&backup_id);
                        let age_desc = backup.age_description();
                        let formatted_size = backup.formatted_size();
                        let backup_desc = backup.description.clone();
                        
                        rsx! {
                            div { 
                                key: "{backup_id}",
                                class: if is_selected {
                                    "backup-item-mini selected"
                                } else {
                                    "backup-item-mini"
                                },
                                onclick: move |_| {
                                    if is_selected {
                                        selected_backup.set(None);
                                    } else {
                                        selected_backup.set(Some(backup_id.clone()));
                                    }
                                },
                                
                                div { class: "backup-info-mini",
                                    div { class: "backup-name", "{backup_desc}" }
                                    div { class: "backup-meta", 
                                        "{age_desc} • {formatted_size}"
                                    }
                                }
                                
                                {if is_selected {
                                    rsx! {
                                        button {
                                            class: "restore-button-mini",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                show_restore_confirm.set(true);
                                            },
                                            "Restore"
                                        }
                                    }
                                } else {
                                    rsx! { span {} }
                                }}
                            }
                        }
                    }).collect();
                    
                    rsx! {
                        div { class: "backups-list-mini",
                            {backup_items.into_iter()}
                            
                            {if available_backups.read().len() > 3 {
                                let remaining_count = available_backups.read().len() - 3;
                                rsx! {
                                    div { class: "backup-show-more",
                                        "... and {remaining_count} more backups"
                                    }
                                }
                            } else {
                                rsx! { span {} }
                            }}
                        }
                    }
                }}
            }
        }
    })
} else {
    None
}}
            div { class: "backup-progress-mini",
            div { class: "progress-text", "{progress_text}" }
            div { class: "progress-bar-mini",
                div { 
                    class: "progress-fill",
                    style: "width: {progress_percentage}%"
                }
            }
        }
    }
})}
                        }
                        
                        // Available backups list
                        div { class: "backup-list-section",
                            h5 { "Available Backups ({available_backups.read().len()})" }
                            
                            
                                            }
                                        }).collect::<Vec<_>>()}
                                        
                                        {if available_backups.read().len() > 3 {
                                            Some(rsx! {
                                                div { class: "backup-show-more",
                                                    "... and {available_backups.read().len() - 3} more backups"
                                                }
                                            })
                                        } else {
                                            None
                                        }}
                                    }
                                })
                            }}
                        }
                    }
                })
            } else {
                None
            }}
            
            // Reset cache option (existing)
            div { class: "advanced-option",
                div { class: "advanced-option-info",
                    h4 { "Reset Installation Cache" }
                    p { "Clears cached files and forces redownload on next launch. Try this if you encounter issues with mods or resources." }
                }
                
                button {
                    class: "advanced-button reset-cache-button",
                    disabled: *is_operating.read(),
                    onclick: move |_| {
                        debug!("Reset cache clicked for installation: {}", installation_id_for_cache);
                        is_operating.set(true);
                        
                        let installation_path_for_cache = installation.installation_path.clone();
                        let mut operation_error_clone = operation_error.clone();
                        let mut is_operating_clone = is_operating.clone();
                        let installation_clone_for_async = installation.clone();
                        let onupdate_clone = onupdate.clone();
                        
                        spawn(async move {
                            let cache_folders = [
                                installation_path_for_cache.join("mods"),
                                installation_path_for_cache.join("resourcepacks"),
                                installation_path_for_cache.join("shaderpacks")
                            ];
                            
                            let mut success = true;
                            
                            for folder in &cache_folders {
                                if folder.exists() {
                                    match std::fs::remove_dir_all(folder) {
                                        Ok(_) => {
                                            if let Err(e) = std::fs::create_dir_all(folder) {
                                                operation_error_clone.set(Some(format!("Failed to recreate folder: {}", e)));
                                                success = false;
                                                break;
                                            }
                                        },
                                        Err(e) => {
                                            operation_error_clone.set(Some(format!("Failed to clear cache: {}", e)));
                                            success = false;
                                            break;
                                        }
                                    }
                                } else {
                                    if let Err(e) = std::fs::create_dir_all(folder) {
                                        operation_error_clone.set(Some(format!("Failed to create folder: {}", e)));
                                        success = false;
                                        break;
                                    }
                                }
                            }
                            
                            is_operating_clone.set(false);
                            
                            if success {
                                let mut installation_for_update = installation_clone_for_async.clone();
                                installation_for_update.installed = false;
                                
                                if let Err(e) = installation_for_update.save() {
                                    operation_error_clone.set(Some(format!("Failed to update installation state: {}", e)));
                                } else {
                                    onupdate_clone.call(installation_for_update);
                                }
                                
                                operation_error_clone.set(Some("Cache successfully reset. You'll need to reinstall the modpack next time you play.".to_string()));
                            }
                        });
                    },
                    "Reset Cache"
                }
            }
        }
        
        // Actions section
        div { class: "settings-section actions",
            h3 { "Actions" }
            
            div { class: "settings-actions",
                // Rename button
                button {
                    class: "settings-action-button rename-button",
                    disabled: *is_operating.read(),
                    onclick: move |_| {
                        new_name.set(installation_name.clone());
                        show_rename_dialog.set(true);
                    },
                    span { class: "action-icon", "✏️" }
                    "Rename Installation"
                }
                
                // Open folder button
                button {
                    class: "settings-action-button folder-button",
                    disabled: *is_operating.read(),
                    onclick: open_folder,
                    span { class: "action-icon", "📂" }
                    "Open Installation Folder"
                }
                
                // Delete button
                button {
                    class: "settings-action-button delete-button",
                    disabled: *is_operating.read(),
                    onclick: move |_| {
                        show_delete_confirm.set(true);
                    },
                    span { class: "action-icon", "🗑️" }
                    "Delete Installation"
                }
            }
        }
        
        // All the existing modals (rename, delete, backup config, restore confirm)
        // Rename dialog
        {if *show_rename_dialog.read() {
            Some(rsx! {
                div { class: "modal-overlay",
                    div { class: "modal-container rename-dialog",
                        div { class: "modal-header",
                            h3 { "Rename Installation" }
                            button { 
                                class: "modal-close",
                                disabled: *is_operating.read(),
                                onclick: move |_| {
                                    if !*is_operating.read() {
                                        show_rename_dialog.set(false);
                                    }
                                },
                                "×"
                            }
                        }
                        
                        div { class: "modal-content",
                            {rename_error.read().as_ref().map(|error| rsx! {
                                div { class: "rename-error", "{error}" }
                            })}
                            
                            div { class: "form-group",
                                label { r#for: "installation-name", "New name:" }
                                input {
                                    id: "installation-name",
                                    r#type: "text",
                                    value: "{new_name}",
                                    maxlength: "25",
                                    disabled: *is_operating.read(),
                                    oninput: move |evt| {
                                        let value = evt.value().clone();
                                        if value.len() <= 25 {
                                            new_name.set(value);
                                        }
                                    },
                                    placeholder: "Enter new installation name"
                                }
                                
                                div { class: "character-counter",
                                    style: if new_name.read().len() > 20 { 
                                        "color: #ff9d93;" 
                                    } else { 
                                        "color: rgba(255, 255, 255, 0.6);" 
                                    },
                                    "{new_name.read().len()}/25"
                                }
                            }
                        }
                        
                        div { class: "modal-footer",
                            button { 
                                class: "cancel-button",
                                disabled: *is_operating.read(),
                                onclick: move |_| {
                                    if !*is_operating.read() {
                                        show_rename_dialog.set(false);
                                    }
                                },
                                "Cancel"
                            }
                            
                            button { 
                                class: "save-button",
                                disabled: *is_operating.read(),
                                onclick: handle_rename,
                                {if *is_operating.read() {
                                    "Saving..."
                                } else {
                                    "Save"
                                }}
                            }
                        }
                    }
                }
            })
        } else {
            None
        }}
        
        // Delete confirmation dialog
        {if *show_delete_confirm.read() {
            Some(rsx! {
                div { class: "modal-overlay",
                    div { class: "modal-container delete-dialog",
                        div { class: "modal-header",
                            h3 { "Delete Installation" }
                            button { 
                                class: "modal-close",
                                disabled: *is_operating.read(),
                                onclick: move |_| {
                                    if !*is_operating.read() {
                                        show_delete_confirm.set(false);
                                    }
                                },
                                "×"
                            }
                        }
                        
                        div { class: "modal-content",
                            p { "Are you sure you want to delete this installation?" }
                            p { class: "delete-warning", "This action cannot be undone!" }
                            p { "Installation: ", strong { "{installation_name}" } }
                        }
                        
                        div { class: "modal-footer",
                            button { 
                                class: "cancel-button",
                                disabled: *is_operating.read(),
                                onclick: move |_| {
                                    if !*is_operating.read() {
                                        show_delete_confirm.set(false);
                                    }
                                },
                                "Cancel"
                            }
                            
                            button { 
                                class: "delete-button",
                                disabled: *is_operating.read(),
                                onclick: handle_delete,
                                {if *is_operating.read() {
                                    "Deleting..."
                                } else {
                                    "Delete"
                                }}
                            }
                        }
                    }
                }
            })
        } else {
            None
        }}
        
        // Backup configuration dialog
        {if *show_backup_config.read() {
            Some(rsx! {
                BackupConfigDialog {
                    config: backup_config,
                    estimated_size: installation.get_backup_size_estimate(&backup_config.read()).unwrap_or(0),
                    onclose: move |_| show_backup_config.set(false),
                    onupdate: move |new_config: BackupConfig| {
                        backup_config.set(new_config);
                    }
                }
            })
        } else {
            None
        }}
        
        // Restore confirmation dialog
        {if *show_restore_confirm.read() {
            Some(rsx! {
                RestoreConfirmDialog {
                    backup_id: selected_backup.read().clone().unwrap_or_default(),
                    backups: available_backups.read().clone(),
                    installation: installation.clone(),
                    onclose: move |_| show_restore_confirm.set(false),
                    onupdate: onupdate.clone()
                }
            })
        } else {
            None
        }}
    }
}
}

// Keep the existing BackupConfigDialog component
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

// New compact restore confirmation dialog
#[component]
fn RestoreConfirmDialog(
    backup_id: String,
    backups: Vec<BackupMetadata>,
    installation: Installation,
    onclose: EventHandler<()>,
    onupdate: EventHandler<Installation>,
) -> Element {
    let backup = backups.iter().find(|b| b.id == backup_id);
    let mut is_restoring = use_signal(|| false);
    let mut restore_error = use_signal(|| Option::<String>::None);
    
    let handle_restore = {
        let installation_clone = installation.clone();
        let backup_id_clone = backup_id.clone();
        let mut is_restoring = is_restoring.clone();
        let mut restore_error = restore_error.clone();
        let onupdate = onupdate.clone();
        let onclose = onclose.clone();
        
        move |_| {
            let mut installation = installation_clone.clone();
            let backup_id = backup_id_clone.clone();
            
            is_restoring.set(true);
            restore_error.set(None);
            
            spawn(async move {
                match installation.restore_from_backup(&backup_id).await {
                    Ok(_) => {
                        onupdate.call(installation);
                        onclose.call(());
                    },
                    Err(e) => {
                        restore_error.set(Some(format!("Failed to restore backup: {}", e)));
                        is_restoring.set(false);
                    }
                }
            });
        }
    };
    
    rsx! {
        div { class: "modal-overlay",
            div { class: "modal-container restore-confirm-dialog",
                div { class: "modal-header",
                    h3 { "Confirm Restore" }
                    button { 
                        class: "modal-close",
                        onclick: move |_| onclose.call(()),
                        "×"
                    }
                }
                
                div { class: "modal-content",
                    if let Some(error) = &*restore_error.read() {
                        div { class: "error-notification",
                            div { class: "error-message", "{error}" }
                        }
                    }
                    
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
                        disabled: *is_restoring.read(),
                        onclick: move |_| onclose.call(()),
                        "Cancel"
                    }
                    
                    button { 
                        class: "restore-confirm-button",
                        disabled: *is_restoring.read(),
                        onclick: handle_restore,
                        if *is_restoring.read() {
                            "Restoring..."
                        } else {
                            "Restore Backup"
                        }
                    }
                }
            }
        }
    }
}
