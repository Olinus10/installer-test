use dioxus::prelude::*;
use crate::installation::{Installation, delete_installation};
use log::{debug, error};
use std::path::PathBuf;

#[component]
pub fn SettingsTab(
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
    let installation_id_for_repair = installation_id.clone();
    
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
    let mut is_operating = use_signal(|| false);
    let mut operation_error = use_signal(|| Option::<String>::None);
    
    // Open folder function - enhanced with debugging and path checks
    let installation_path_for_folder = installation.installation_path.clone();
   let open_folder = move |_| {
    let mut path = installation_path_for_folder.clone();
    
    // Add extensive debugging for path troubleshooting
    debug!("Opening installation folder: {:?}", path);
    debug!("Path exists: {}", path.exists());
    debug!("Path is directory: {}", path.is_dir());
    debug!("Path parent: {:?}", path.parent());
    
    // Normalize the path by converting to a canonical path
    // This ensures proper path separators for the platform
    match path.canonicalize() {
        Ok(canonical) => {
            debug!("Canonical path: {:?}", canonical);
            path = canonical;
        },
        Err(e) => {
            debug!("Failed to canonicalize path: {}", e);
            // Continue with the original path
        }
    }
    
    debug!("Final path to open: {:?}", path);
    
    // Check if path exists
    if !path.exists() {
        debug!("Installation path does not exist: {:?}", path);
        operation_error.set(Some(format!("Folder does not exist: {:?}", path)));
        return;
    }
    
    // Launch appropriate command based on OS
    #[cfg(target_os = "windows")]
    let result = {
        // Convert to a proper Windows-style path string
        let path_str = path.to_string_lossy().replace("/", "\\");
        debug!("Windows path string: {}", path_str);
        
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
    
    // Handle command result
    if let Err(e) = result {
        debug!("Failed to open installation folder: {}", e);
        operation_error.set(Some(format!("Failed to open folder: {}", e)));
    } else {
        debug!("Successfully opened folder");
    }
};
    
    // Handle rename
    let installation_for_rename = installation.clone();
    let handle_rename = move |_| {
        let mut installation_copy = installation_for_rename.clone();
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
                // Call the update handler with the updated installation
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
    is_operating.set(true);
    
    spawn(async move {
        match delete_installation(&id_to_delete) {
            Ok(_) => {
                debug!("Successfully deleted installation: {}", id_to_delete);
                // Call the ondelete handler to navigate back to home
                delete_handler.call(());
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
                    
                    div { class: "info-row",
                        div { class: "info-label", "Path: " }
                        div { class: "info-value truncate-path", "{installation_path_display}" }
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
                            if let Some(launch_date) = last_launch {
                                {launch_date.format("%B %d, %Y").to_string()}
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
                debug!("Reset cache clicked for installation: {}", installation_id_for_cache);
                is_operating.set(true);
                
                let installation_path_for_cache = installation.installation_path.clone();
                let mut operation_error_clone = operation_error.clone();
                let mut is_operating_clone = is_operating.clone();
                
                spawn(async move {
                    // Define the folders to clear
                    let cache_folders = [
                        installation_path_for_cache.join("mods"),
                        installation_path_for_cache.join("resourcepacks"),
                        installation_path_for_cache.join("shaderpacks")
                    ];
                    
                    debug!("Clearing cache folders: {:?}", cache_folders);
                    
                    let mut success = true;
                    
                    // Delete the content of each folder
                    for folder in &cache_folders {
                        if folder.exists() {
                            match std::fs::remove_dir_all(folder) {
                                Ok(_) => {
                                    debug!("Removed folder: {:?}", folder);
                                    // Recreate the empty folder
                                    if let Err(e) = std::fs::create_dir_all(folder) {
                                        error!("Failed to recreate folder {:?}: {}", folder, e);
                                        operation_error_clone.set(Some(format!("Failed to recreate folder: {}", e)));
                                        success = false;
                                        break;
                                    }
                                },
                                Err(e) => {
                                    error!("Failed to remove folder {:?}: {}", folder, e);
                                    operation_error_clone.set(Some(format!("Failed to clear cache: {}", e)));
                                    success = false;
                                    break;
                                }
                            }
                        } else {
                            // Create the folder if it doesn't exist
                            if let Err(e) = std::fs::create_dir_all(folder) {
                                error!("Failed to create folder {:?}: {}", folder, e);
                                operation_error_clone.set(Some(format!("Failed to create folder: {}", e)));
                                success = false;
                                break;
                            }
                        }
                    }
                    
                    is_operating_clone.set(false);
                    
                    // Display success message if everything went well
                    if success {
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
