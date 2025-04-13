use dioxus::prelude::*;
use crate::installation::Installation;
use log::debug;

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
        
        // Save changes
        match installation_copy.save() {
            Ok(_) => {
                debug!("Renamed installation to: {}", installation_copy.name);
                show_rename_dialog.set(false);
                // Ideally we would update the UI to show the new name
                // For a complete implementation, we might want to refresh the installation
            },
            Err(e) => {
                debug!("Failed to rename installation: {}", e);
                rename_error.set(Some(format!("Failed to rename installation: {}", e)));
            }
        }
    };
    
    rsx! {
        div { class: "settings-tab",
            h2 { "Installation Settings" }
            
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
            
            // Actions section
            div { class: "settings-section actions",
                h3 { "Actions" }
                
                div { class: "settings-actions",
                    // Rename button
                    button {
                        class: "settings-action-button rename-button",
                        onclick: move |_| {
                            new_name.set(installation.name.clone());
                            show_rename_dialog.set(true);
                        },
                        span { class: "action-icon", "✏️" }
                        "Rename Installation"
