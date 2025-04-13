use dioxus::prelude::*;
use crate::installation::{Installation, delete_installation};
use log::{debug, error};

#[component]
pub fn SettingsTab(
    installation: Installation,
    installation_id: String,
    ondelete: EventHandler<()>,
) -> Element {
    // State for rename dialog
    let mut show_rename_dialog = use_signal(|| false);
    let mut new_name = use_signal(|| installation.name.clone());
    let mut rename_error = use_signal(|| Option::<String>::None);
    
    // State for delete confirmation
    let mut show_delete_confirm = use_signal(|| false);
    
    // State for operation status
    let mut is_operating = use_signal(|| false);
    let mut operation_error = use_signal(|| Option::<String>::None);
    
    // Open folder function
    let open_folder = move |_| {
        let path = &installation.installation_path;
        debug!("Opening installation folder: {:?}", path);
        
        #[cfg(target_os = "windows")]
        let result = std::process::Command::new("explorer")
            .arg(path)
            .spawn();
            
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
    let handle_rename = move |_| {
        let mut installation_copy = installation.clone();
        installation_copy.name = new_name.read().clone();
        
        // Validate name
        if installation_copy.name.trim().is_empty() {
            rename_error.set(Some("Installation name cannot be empty".to_string()));
            return;
        }
        
        is_operating.set(true);
        
        // Save changes
        match installation_copy.save() {
            Ok(_) => {
                debug!("Renamed installation to: {}", installation_copy.name);
                show_rename_dialog.set(false);
                is_operating.set(false);
                // Ideally we would update the UI to show the new name
                // For a complete implementation, we might want to refresh the installation
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
        let id_to_delete = installation_id.clone();
        is_operating.set(true);
        
        spawn(async move {
            match delete_installation(&id_to_delete) {
                Ok(_) => {
                    debug!("Successfully deleted installation: {}", id_to_delete);
                    // Call the ondelete handler to notify parent component
                    ondelete.call(());
                },
                Err(e) => {
                    error!("Failed to delete installation: {}", e);
                    operation_error.set(Some(format!("Failed to delete installation: {}", e)));
                    is_operating.set(false);
                }
            }
        });
    };
    
    rsx! {
        div { class: "settings-tab",
            h2 { "Installation Settings" }
            
            // Display operation error if any
            if let Some(error) = &*operation_error.read() {
                div { class: "error-notification settings-error",
                    div { class: "error-message", "{error}" }
                    button { 
                        class: "error-close",
                        onclick: move |_| operation_error.set(None),
                        "×"
                    }
                }
            }
            
            // Installation information section
            div { class: "settings-section installation-info",
                h3 { "Installation Information" }
                
                div { class: "info-grid",
                    div { class: "info-row",
                        div { class: "info-label", "Name:" }
                        div { class: "info-value", "{installation.name}" }
                    }
                    
                    div { class: "info-row",
                        div { class: "info-label", "Created:" }
                        div { class: "info-value", "{installation.created_at.format(\"%B %d, %Y\")}" }
                    }
                    
                    div { class: "info-row",
                        div { class: "info-label", "Last Used:" }
                        div { class: "info-value", "{installation.last_used.format(\"%B %d, %Y\")}" }
                    }
                    
                    div { class: "info-row",
                        div { class: "info-label", "Minecraft:" }
                        div { class: "info-value", "{installation.minecraft_version}" }
                    }
                    
                    div { class: "info-row",
                        div { class: "info-label", "Loader:" }
                        div { class: "info-value", "{installation.loader_type} {installation.loader_version}" }
                    }
                    
                    div { class: "info-row",
                        div { class: "info-label", "Launcher:" }
                        div { class: "info-value", "{installation.launcher_type}" }
                    }
                    
                    div { class: "info-row",
                        div { class: "info-label", "Path:" }
                        div { class: "info-value truncate-path", "{installation.installation_path.display()}" }
                    }
                }
            }
            
            // Usage statistics section
            div { class: "settings-section usage-stats",
                h3 { "Usage Statistics" }
                
                div { class: "stats-grid",
                    div { class: "stat-item",
                        div { class: "stat-value", "{installation.total_launches}" }
                        div { class: "stat-label", "Total Launches" }
                    }
                    
                    div { class: "stat-item",
                        div { class: "stat-value",
                            if let Some(last_launch) = installation.last_launch {
                                {last_launch.format("%B %d, %Y").to_string()}
                            } else {
                                {"Never".to_string()}
                            }
                        }
                        div { class: "stat-label", "Last Launch" }
                    }
                }
            }
            
            // Advanced section
            div { class: "settings-section advanced-settings",
                h3 { "Advanced" }
                
                div { class: "advanced-description",
                    p { "These settings allow you to directly manage your installation. Use with caution." }
                }
                
                // Reset cache option
                div { class: "advanced-option",
                    div { class: "advanced-option-info",
                        h4 { "Reset Installation Cache" }
                        p { "Clears cached files and forces redownload on next launch. Try this if you encounter issues with mods or resources." }
                    }
                    
                    button {
                        class: "advanced-button reset-cache-button",
                        disabled: *is_operating.read(),
                        onclick: move |_| {
                            debug!("Reset cache clicked for installation: {}", installation.id);
                            // Implementation would go here - for now just a placeholder
                            operation_error.set(Some("This functionality is not yet implemented".to_string()));
                        },
                        "Reset Cache"
                    }
                }
                
                // Repair installation option
                div { class: "advanced-option",
                    div { class: "advanced-option-info",
                        h4 { "Repair Installation" }
                        p { "Attempts to fix a broken installation by verifying and repairing essential files." }
                    }
                    
                    button {
                        class: "advanced-button repair-button",
                        disabled: *is_operating.read(),
                        onclick: move |_| {
                            debug!("Repair installation clicked for: {}", installation.id);
                            // Implementation would go here - for now just a placeholder
                            operation_error.set(Some("This functionality is not yet implemented".to_string()));
                        },
                        "Repair Installation"
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
                            new_name.set(installation.name.clone());
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
            
            // Rename dialog
            if *show_rename_dialog.read() {
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
                            // Error message if any
                            if let Some(error) = &*rename_error.read() {
                                div { class: "rename-error", "{error}" }
                            }
                            
                            div { class: "form-group",
                                label { r#for: "installation-name", "New name:" }
                                input {
                                    id: "installation-name",
                                    r#type: "text",
                                    value: "{new_name}",
                                    disabled: *is_operating.read(),
                                    oninput: move |evt| new_name.set(evt.value().clone()),
                                    placeholder: "Enter new installation name"
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
                                if *is_operating.read() {
                                    "Saving..."
                                } else {
                                    "Save"
                                }
                            }
                        }
                    }
                }
            }
            
            // Delete confirmation dialog
            if *show_delete_confirm.read() {
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
                            p { "Installation: ", strong { "{installation.name}" } }
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
                                if *is_operating.read() {
                                    "Deleting..."
                                } else {
                                    "Delete"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
