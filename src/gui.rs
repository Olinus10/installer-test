use std::{collections::BTreeMap, path::PathBuf};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use dioxus::prelude::*;
use log::{error, debug};
use modal::ModalContext;
use modal::Modal; 
use std::sync::mpsc;

use crate::{get_app_data, get_installed_packs, get_launcher, uninstall, InstallerProfile, Launcher, PackName, Changelog,launcher::launch_modpack};
use crate::{Installation, Preset, UniversalManifest};
use crate::preset;
use crate::{get_active_account, get_all_accounts, authenticate, is_authenticated};

mod modal;

const HEADER_FONT: &str = "\"HEADER_FONT\"";
const REGULAR_FONT: &str = "\"REGULAR_FONT\"";

#[derive(Debug, Clone)]
struct TabInfo {
    color: String,
    title: String, 
    background: String,
    settings_background: String,
    modpacks: Vec<InstallerProfile>,
}

#[component]
fn BackgroundParticles() -> Element {
    rsx! {
        div { class: "particles-container",
            // Generate particles with different sizes, colors and animations
            for i in 0..20 {
                {
                    let size = 4 + (i % 6);
                    let delay = i as f32 * 0.5;
                    let duration = 10.0 + (i % 10) as f32;
                    let left = 5 + (i * 5) % 95;
                    
                    let particle_class = match i % 3 {
                        0 => "particle",
                        1 => "particle purple",
                        _ => "particle green",
                    };
                    
                    let animation = if i % 2 == 0 { "float" } else { "float-horizontal" };
                    
                    rsx! {
                        div {
                            class: "{particle_class}",
                            style: "width: {size}px; height: {size}px; left: {left}%; 
                                bottom: -50px; opacity: 0.6; 
                                animation: {animation} {duration}s ease-in infinite {delay}s;"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ErrorNotification(message: String, on_close: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { class: "error-notification",
            div { class: "error-message", "{message}" }
            button { 
                class: "error-close",
                onclick: move |evt| on_close.call(evt),
                "×"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthStatus {
    Authenticated,  // User already authenticated
    NeedsAuth,      // User needs to authenticate first
}

// Helper function to check auth status of a profile
pub fn get_auth_status() -> AuthStatus {
    // Only check authentication if UI is fully loaded
    if !app_fully_initialized() {
        return AuthStatus::NeedsAuth; // Default to needing auth during initialization
    }
    
    if crate::launcher::microsoft_auth::MicrosoftAuth::is_authenticated() {
        AuthStatus::Authenticated
    } else {
        AuthStatus::NeedsAuth
    }
}

// Add a function to check if app is fully initialized
fn app_fully_initialized() -> bool {
    // Check if pages are loaded, etc.
    // Return false during initial loading
    true // Change this based on actual state
}

// Thread-safe play button handler
pub fn handle_play_click(uuid: String, error_signal: &Signal<Option<String>>) {
    debug!("Play button clicked for modpack: {}", uuid);
    
    // Create a channel to communicate back to the main thread
    let (error_tx, error_rx) = mpsc::channel::<String>();
    
    // Clone error_signal before moving to thread
    let mut error_signal_clone = error_signal.clone();
    
    // Check authentication status
    match get_auth_status() {
        AuthStatus::Authenticated => {
            // User is already authenticated, launch the game
            let uuid_clone = uuid.clone();
            std::thread::spawn(move || {
                match crate::launcher::microsoft_auth::MicrosoftAuth::launch_minecraft(&uuid_clone) {
                    Ok(_) => {
                        debug!("Successfully launched modpack: {}", uuid_clone);
                    },
                    Err(e) => {
                        error!("Failed to launch modpack: {}", e);
                        let _ = error_tx.send(format!("Failed to launch modpack: {}", e));
                    }
                }
            });
        },
        AuthStatus::NeedsAuth => {
            // User needs to authenticate first
            let uuid_clone = uuid.clone();
            let error_tx_clone = error_tx.clone();
            std::thread::spawn(move || {
                match crate::launcher::microsoft_auth::MicrosoftAuth::authenticate() {
                    Ok(_) => {
                        debug!("Authentication successful, now launching modpack: {}", uuid_clone);
                        // After successful authentication, launch the game
                        match crate::launcher::microsoft_auth::MicrosoftAuth::launch_minecraft(&uuid_clone) {
                            Ok(_) => {
                                debug!("Successfully launched modpack after authentication: {}", uuid_clone);
                            },
                            Err(e) => {
                                error!("Failed to launch modpack after authentication: {}", e);
                                let _ = error_tx_clone.send(format!("Failed to launch modpack: {}", e));
                            }
                        }
                    },
                    Err(e) => {
                        error!("Authentication failed: {}", e);
                        let _ = error_tx.send(format!("Microsoft authentication failed: {}", e));
                    }
                }
            });
        }
    }
    
    // Create a task to check for errors from the background thread
    spawn(async move {
        if let Ok(error_message) = error_rx.recv() {
            error_signal_clone.set(Some(error_message));
        }
    });
}

// PlayButton component
#[component]
pub fn PlayButton(
    uuid: String,
    disabled: bool,
    auth_status: Option<AuthStatus>,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    // Check the current authentication status if not provided
    let status = auth_status.unwrap_or_else(get_auth_status);
    
    // Get username if authenticated
    let username_display = if status == AuthStatus::Authenticated {
        if let Some(username) = crate::launcher::microsoft_auth::MicrosoftAuth::get_username() {
            Some(format!("Playing as {}", username))
        } else {
            None
        }
    } else {
        None
    };
    
    rsx! {
        div { class: "play-button-container",
            button {
                class: if status == AuthStatus::Authenticated {
                    "main-play-button authenticated"
                } else {
                    "main-play-button needs-auth"
                },
                disabled: disabled,
                onclick: move |evt| onclick.call(evt),
                if status == AuthStatus::Authenticated {
                    "PLAY"
                } else {
                    "LOGIN WITH MICROSOFT"
                }
            }
            
            // Show authentication status if available
            if let Some(username) = username_display {
                div { class: "auth-status", "{username}" }
            }
            
            // Help text based on auth status
            div { class: "auth-info",
                if status == AuthStatus::Authenticated {
                    "Click to launch Minecraft directly"
                } else {
                    "Microsoft account required to play"
                }
            }
        }
    }
}

#[component]
fn ChangelogSection(changelog: Option<Changelog>) -> Element {
    if let Some(changelog_data) = changelog {
        if changelog_data.entries.is_empty() {
            return None;
        }
        
        rsx! {
            div { class: "changelog-container",
                div { class: "section-divider with-title", 
                    span { class: "divider-title", "LATEST CHANGES" }
                }
                
                div { class: "changelog-entries",
                    for (index, entry) in changelog_data.entries.iter().enumerate().take(5) {
                        div { 
                            class: "changelog-entry",
                            "data-importance": "{entry.importance.clone().unwrap_or_else(|| String::from(\"normal\"))}",
                            
                            div { class: "changelog-header",
                                h3 { class: "changelog-title", "{entry.title}" }
                                
                                if let Some(version) = &entry.version {
                                    span { class: "changelog-version", "v{version}" }
                                }
                                
                                if let Some(date) = &entry.date {
                                    span { class: "changelog-date", "{date}" }
                                }
                            }
                            
                            div { 
                                class: "changelog-content",
                                dangerous_inner_html: "{entry.contents}"
                            }
                            
                            // Show divider between entries except for the last one
                            if index < changelog_data.entries.len() - 1 && index < 4 {
                                div { class: "entry-divider" }
                            }
                        }
                    }
                    
                    // Show "View all changes" button if more than 5 entries
                    if changelog_data.entries.len() > 5 {
                        div { class: "view-all-changes",
                            button { class: "view-all-button",
                                "View All Changes"
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Return empty div while loading
        rsx! { div { class: "changelog-loading" } }
    }
}

// Add this new component for the footer with Discord button
#[component]
fn Footer() -> Element {
    rsx! {
        footer { class: "app-footer",
            div { class: "footer-content",
                div { class: "footer-section",
                    h3 { class: "footer-heading", "Community" }
                    a { 
                        class: "discord-button",
                        href: "https://discord.gg/olinus-corner-778965021656743966",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        
                        // Discord logo (simplified SVG as inline content)
                        svg {
    class: "discord-logo",
    xmlns: "http://www.w3.org/2000/svg",
    width: "24",
    height: "24",
    view_box: "0 0 24 24",  // Changed viewBox to view_box
    fill: "currentColor",
    
    path {
        d: "M19.54 0c1.356 0 2.46 1.104 2.46 2.472v21.528l-2.58-2.28-1.452-1.344-1.536-1.428.636 2.22h-13.608c-1.356 0-2.46-1.104-2.46-2.472v-16.224c0-1.368 1.104-2.472 2.46-2.472h16.08zm-4.632 15.672c2.652-.084 3.672-1.824 3.672-1.824 0-3.864-1.728-6.996-1.728-6.996-1.728-1.296-3.372-1.26-3.372-1.26l-.168.192c2.04.624 2.988 1.524 2.988 1.524-1.248-.684-2.472-1.02-3.612-1.152-.864-.096-1.692-.072-2.424.024l-.204.024c-.42.036-1.44.192-2.724.756-.444.204-.708.348-.708.348s.996-.948 3.156-1.572l-.12-.144s-1.644-.036-3.372 1.26c0 0-1.728 3.132-1.728 6.996 0 0 1.008 1.74 3.66 1.824 0 0 .444-.54.804-.996-1.524-.456-2.1-1.416-2.1-1.416l.336.204.048.036.047.027.014.006.047.027c.3.168.6.3.876.408.492.192 1.08.384 1.764.516.9.168 1.956.228 3.108.012.564-.096 1.14-.264 1.74-.516.42-.156.888-.384 1.38-.708 0 0-.6.984-2.172 1.428.36.456.792.972.792.972zm-5.58-5.604c-.684 0-1.224.6-1.224 1.332 0 .732.552 1.332 1.224 1.332.684 0 1.224-.6 1.224-1.332.012-.732-.54-1.332-1.224-1.332zm4.38 0c-.684 0-1.224.6-1.224 1.332 0 .732.552 1.332 1.224 1.332.684 0 1.224-.6 1.224-1.332 0-.732-.54-1.332-1.224-1.332z"
    }
                        }
                        
                        span { "Join our Discord" }
                    }
                }
                
                div { class: "footer-section",
                    h3 { class: "footer-heading", "About" }
                    p { class: "footer-text", 
                        "The Wynncraft Overhaul Installer provides easy access to modpacks that enhance your Wynncraft experience."
                    }
                }
                
                div { class: "footer-section",
                    h3 { class: "footer-heading", "Legal" }
                    p { class: "footer-text", 
                        "All modpacks are made by the community and are not affiliated with Wynncraft."
                    }
                }
            }
            
            div { class: "footer-bottom",
                p { class: "copyright", "© 2023-2025 Majestic Overhaul. CC BY-NC-SA 4.0." }
            }
        }
    }
}

// Home Page component with redundancy removed
#[component]
fn NewHomePage(
    installations: Signal<Vec<Installation>>,
    error_signal: Signal<Option<String>>,
) -> Element {
    let has_installations = !installations().is_empty();
    let latest_installation = installations().first().cloned();
    
    // State for creation dialog
    let mut show_creation_dialog = use_signal(|| false);
    
    // Authentication status check
    let auth_status = get_auth_status();
    let username = if auth_status == AuthStatus::Authenticated {
        crate::launcher::microsoft_auth::MicrosoftAuth::get_username()
    } else {
        None
    };
    
    rsx! {
        div { class: "home-container",
            if has_installations {
                // Welcome header with username if available
                div { class: "welcome-header",
                    if let Some(name) = username {
                        h1 { "Welcome back, {name}!" }
                    } else {
                        h1 { "Welcome back!" }
                    }
                }
                
                // Statistics display
                StatisticsDisplay {}
                
                // Section divider
                div { class: "section-divider with-title", 
                    span { class: "divider-title", "YOUR INSTALLATIONS" }
                }
                
                // Play button for latest installation
                if let Some(installation) = latest_installation {
                    div { class: "main-play-container",
                        PlayButton {
                            uuid: installation.id.clone(),
                            disabled: false,
                            auth_status: Some(auth_status),
                            onclick: move |_| {
                                let installation_id = installation.id.clone();
                                handle_play_click(installation_id, &error_signal);
                            }
                        }
                    }
                }
                
                // List of all installations
                div { class: "installations-grid",
                    for installation in installations() {
                        InstallationCard { 
                            installation: installation.clone(),
                            onclick: move |id| {
                                // Navigate to installation page
                                // This will depend on your navigation system
                            }
                        }
                    }
                    
                    // "Create new" card
                    div { class: "installation-card new-installation",
                        div { class: "installation-card-content", 
                            div { class: "installation-card-icon", "+" }
                            h3 { "Create New Installation" }
                            
                            button { 
                                class: "create-button",
                                onclick: move |_| {
                                    show_creation_dialog.set(true);
                                },
                                "Start"
                            }
                        }
                    }
                }
            } else {
                // First-time user experience
                div { class: "welcome-container",
                    h1 { "Welcome to Wynncraft Overhaul" }
                    p { "Get started by creating your first custom installation." }
                    
                    button {
                        class: "main-install-button",
                        onclick: move |_| {
                            show_creation_dialog.set(true);
                        },
                        "Create Installation"
                    }
                }
            }
            
            // Creation dialog
            if *show_creation_dialog.read() {
                InstallationCreationWizard {
                    onclose: move |_| {
                        show_creation_dialog.set(false);
                    },
                    oncreate: move |new_installation| {
                        // Add the new installation and refresh
                        installations.with_mut(|list| {
                            list.insert(0, new_installation);
                        });
                        show_creation_dialog.set(false);
                    }
                }
            }
        }
    }
}
// Special value for home page
const HOME_PAGE: usize = usize::MAX;


#[derive(PartialEq, Props, Clone)]
struct InstallationCardProps {
    installation: Installation,
    onclick: EventHandler<String>,
}

#[component]
fn InstallationCard(props: InstallationCardProps) -> Element {
    let installation = props.installation.clone();
    
    // Format last played date
    let last_played = installation.last_launch.map(|dt| {
        // Format date as readable string
        dt.format("%B %d, %Y").to_string()
    });
    
    rsx! {
        div { 
            class: "installation-card",
            onclick: move |_| {
                props.onclick.call(installation.id.clone());
            },
            
            div { class: "installation-card-header",
                h3 { "{installation.name}" }
                
                if installation.update_available {
                    span { class: "update-badge", "Update Available" }
                }
            }
            
            div { class: "installation-card-details",
                div { class: "detail-item",
                    span { class: "detail-label", "Minecraft:" }
                    span { class: "detail-value", "{installation.minecraft_version}" }
                }
                
                div { class: "detail-item",
                    span { class: "detail-label", "Loader:" }
                    span { class: "detail-value", "{installation.loader_type} {installation.loader_version}" }
                }
                
                div { class: "detail-item",
                    span { class: "detail-label", "Last Played:" }
                    span { class: "detail-value",
                        if let Some(last) = last_played {
                            {last}
                        } else {
                            {"Never"}
                        }
                    }
                }
            }
            
            div { class: "installation-card-actions",
                button { 
                    class: "play-button",
                    "Play"
                }
                
                button { 
                    class: "settings-button",
                    "Settings"
                }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct InstallationCreationWizardProps {
    onclose: EventHandler<()>,
    oncreate: EventHandler<Installation>,
}

#[component]
fn InstallationCreationWizard(props: InstallationCreationWizardProps) -> Element {
    // State for wizard
    let mut current_step = use_signal(|| 0);
    let mut name = use_signal(|| "My Wynncraft Installation".to_string());
    let mut selected_preset_id = use_signal(|| Option::<String>::None);
    let mut minecraft_version = use_signal(|| "1.20.4".to_string());
    let mut loader_type = use_signal(|| "fabric".to_string());
    let mut memory_allocation = use_signal(|| 3072);
    
    // Resource for presets
    let presets: Resource<Vec<Preset>> = use_resource(move || async {
        match crate::preset::load_presets(&crate::CachedHttpClient::new(), None).await {
            Ok(presets) => presets,
            Err(_) => Vec::new(),
        }
    });
    
    // Resource for universal manifest
    let universal_manifest: Resource<Option<UniversalManifest>> = use_resource(move || async {
        match crate::universal::load_universal_manifest(&crate::CachedHttpClient::new(), None).await {
            Ok(manifest) => Some(manifest),
            Err(_) => None,
        }
    });
    
    // Step titles for progress display
    let step_titles = vec![
        "Basic Info", 
        "Select Preset", 
        "Performance",
        "Review"
    ];
    
    // Function to create the installation
    let create_installation = move || {
        // Find the selected preset
        let preset = if let Some(preset_id) = &*selected_preset_id.read() {
            if let Some(presets) = presets.read() {
                preset::find_preset_by_id(presets, preset_id)
            } else {
                None
            }
        } else {
            None
        };
        
        // Get the loader version from the universal manifest
        let loader_version = if let Some(manifest) = universal_manifest.read().as_ref() {
            manifest.loader.version.clone()
        } else {
            "0.15.3".to_string() // Default loader version
        };
        
        // Create the installation
        if let Some(preset) = preset {
            let installation = Installation::new_from_preset(
                name.read().clone(),
                &preset,
                minecraft_version.read().clone(),
                loader_type.read().clone(),
                loader_version,
                "vanilla".to_string(), // Default to vanilla launcher
                "1.0.0".to_string(),   // Default universal version
            );
            
            // Register the installation
            if let Err(e) = crate::installation::register_installation(&installation) {
                error!("Failed to register installation: {}", e);
                // Continue anyway - we'll return the installation
            }
            
            // Save the installation
            if let Err(e) = installation.save() {
                error!("Failed to save installation: {}", e);
                // Continue anyway
            }
            
            // Return the new installation
            props.oncreate.call(installation);
        } else {
            // Create custom installation without preset
            let installation = Installation::new_custom(
                name.read().clone(),
                minecraft_version.read().clone(),
                loader_type.read().clone(),
                loader_version,
                "vanilla".to_string(),
                "1.0.0".to_string(),
            );
            
            // Register and save the installation
            if let Err(e) = crate::installation::register_installation(&installation) {
                error!("Failed to register installation: {}", e);
            }
            
            if let Err(e) = installation.save() {
                error!("Failed to save installation: {}", e);
            }
            
            props.oncreate.call(installation);
        }
    };
    
    rsx! {
        div { class: "wizard-overlay",
            div { class: "wizard-container",
                // Wizard header
                div { class: "wizard-header",
                    h2 { "Create New Installation" }
                    
                    // Progress steps
                    div { class: "wizard-progress",
                        for (index, title) in step_titles.iter().enumerate() {
                            div { 
                                class: if index == *current_step.read() {
                                    "wizard-step active"
                                } else if index < *current_step.read() {
                                    "wizard-step completed"
                                } else {
                                    "wizard-step"
                                },
                                
                                div { class: "step-number", "{index + 1}" }
                                div { class: "step-title", "{title}" }
                            }
                        }
                    }
                }
                
                // Wizard content - different for each step
                div { class: "wizard-content",
                    match *current_step.read() {
                        0 => rsx! {
                            // Step 1: Basic Info
                            div { class: "wizard-step-content",
                                h3 { "Give your installation a name" }
                                
                                div { class: "form-group",
                                    label { for: "installation-name", "Installation Name:" }
                                    input {
                                        id: "installation-name",
                                        r#type: "text",
                                        value: "{name}",
                                        oninput: move |evt| {
                                            name.set(evt.value.clone());
                                        }
                                    }
                                }
                                
                                div { class: "form-group",
                                    label { for: "minecraft-version", "Minecraft Version:" }
                                    select {
                                        id: "minecraft-version",
                                        value: "{minecraft_version}",
                                        onchange: move |evt| {
                                            minecraft_version.set(evt.value.clone());
                                        },
                                        
                                        option { value: "1.20.4", "1.20.4" }
                                        option { value: "1.19.4", "1.19.4" }
                                        option { value: "1.18.2", "1.18.2" }
                                    }
                                }
                                
                                div { class: "form-group",
                                    label { for: "loader-type", "Mod Loader:" }
                                    select {
                                        id: "loader-type",
                                        value: "{loader_type}",
                                        onchange: move |evt| {
                                            loader_type.set(evt.value.clone());
                                        },
                                        
                                        option { value: "fabric", "Fabric" }
                                        option { value: "quilt", "Quilt" }
                                    }
                                }
                            }
                        },
                        1 => rsx! {
                            // Step 2: Select Preset
                            div { class: "wizard-step-content",
                                h3 { "Choose a preset configuration" }
                                p { "Presets determine which mods are enabled by default." }
                                
                                if let Some(presets_list) = presets.read() {
                                    if presets_list.is_empty() {
                                        div { class: "loading-message", "Loading presets..." }
                                    } else {
                                        div { class: "presets-grid",
                                            for preset in presets_list {
                                                {
                                                    let preset_id = preset.id.clone();
                                                    let is_selected = selected_preset_id.read().as_ref().map_or(false, |id| id == &preset_id);
                                                    
                                                    rsx! {
                                                        div { 
                                                            class: if is_selected {
                                                                "preset-card selected"
                                                            } else {
                                                                "preset-card"
                                                            },
                                                            onclick: move |_| {
                                                                selected_preset_id.set(Some(preset_id.clone()));
                                                            },
                                                            
                                                            h4 { "{preset.name}" }
                                                            p { "{preset.description}" }
                                                            
                                                            if let Some(author) = &preset.author {
                                                                div { class: "preset-author", "By: {author}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            // Custom preset option
                                            div { 
                                                class: if selected_preset_id.read().is_none() {
                                                    "preset-card selected"
                                                } else {
                                                    "preset-card"
                                                },
                                                onclick: move |_| {
                                                    selected_preset_id.set(None);
                                                },
                                                
                                                h4 { "Custom Configuration" }
                                                p { "Start with a minimal setup and customize everything yourself." }
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "loading-message", "Loading presets..." }
                                }
                            }
                        },
                        2 => rsx! {
                            // Step 3: Performance Settings
                            div { class: "wizard-step-content",
                                h3 { "Performance Settings" }
                                p { "Configure memory allocation and other performance settings." }
                                
                                div { class: "form-group",
                                    label { for: "memory-allocation", 
                                        "Memory Allocation: {memory_allocation} MB"
                                    }
                                    input {
                                        id: "memory-allocation",
                                        r#type: "range",
                                        min: "1024",
                                        max: "8192",
                                        step: "512",
                                        value: "{memory_allocation}",
                                        oninput: move |evt| {
                                            if let Ok(value) = evt.value.parse::<i32>() {
                                                memory_allocation.set(value);
                                            }
                                        }
                                    }
                                    div { class: "memory-markers",
                                        span { "1 GB" }
                                        span { "4 GB" }
                                        span { "8 GB" }
                                    }
                                }
                                
                                // Show preset recommended settings if applicable
                                if let Some(preset_id) = &*selected_preset_id.read() {
                                    if let Some(presets_list) = presets.read() {
                                        if let Some(preset) = preset::find_preset_by_id(presets_list, preset_id) {
                                            if let Some(rec_memory) = preset.recommended_memory {
                                                div { class: "recommended-setting",
                                                    "Recommended memory for this preset: {rec_memory} MB"
                                                    
                                                    button {
                                                        class: "apply-recommended-button",
                                                        onclick: move |_| {
                                                            memory_allocation.set(rec_memory);
                                                        },
                                                        "Apply"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        3 => rsx! {
                            // Step 4: Review
                            div { class: "wizard-step-content",
                                h3 { "Review Your Installation" }
                                
                                div { class: "review-container",
                                    div { class: "review-item",
                                        div { class: "review-label", "Name:" }
                                        div { class: "review-value", "{name}" }
                                    }
                                    
                                    div { class: "review-item",
                                        div { class: "review-label", "Minecraft Version:" }
                                        div { class: "review-value", "{minecraft_version}" }
                                    }
                                    
                                    div { class: "review-item",
                                        div { class: "review-label", "Mod Loader:" }
                                        div { class: "review-value", "{loader_type}" }
                                    }
                                    
                                    div { class: "review-item",
                                        div { class: "review-label", "Preset:" }
                                        div { class: "review-value", 
                                            if let Some(preset_id) = &*selected_preset_id.read() {
                                                if let Some(presets_list) = presets.read() {
                                                    if let Some(preset) = preset::find_preset_by_id(presets_list, preset_id) {
                                                        preset.name
                                                    } else {
                                                        "Custom Configuration".to_string()
                                                    }
                                                } else {
                                                    "Custom Configuration".to_string()
                                                }
                                            } else {
                                                "Custom Configuration".to_string()
                                            }
                                        }
                                    }
                                    
                                    div { class: "review-item",
                                        div { class: "review-label", "Memory Allocation:" }
                                        div { class: "review-value", "{memory_allocation} MB" }
                                    }
                                }
                                
                                div { class: "summary-message",
                                    "Your installation will be created with these settings. You can modify them later in the installation settings."
                                }
                            }
                        },
                        _ => rsx! {
                            div { "Unknown step" }
                        }
                    }
                }
                
                // Wizard footer with navigation buttons
                div { class: "wizard-footer",
                    button {
                        class: "cancel-button",
                        onclick: move |_| {
                            props.onclose.call(());
                        },
                        "Cancel"
                    }
                    
                    div { class: "navigation-buttons",
                        if *current_step.read() > 0 {
                            button {
                                class: "back-button",
                                onclick: move |_| {
                                    current_step.with_mut(|step| {
                                        if *step > 0 {
                                            *step -= 1;
                                        }
                                    });
                                },
                                "Back"
                            }
                        }
                        
                        button {
                            class: if *current_step.read() == step_titles.len() - 1 {
                                "next-button create-button"
                            } else {
                                "next-button"
                            },
                            onclick: move |_| {
                                if *current_step.read() < step_titles.len() - 1 {
                                    current_step.with_mut(|step| {
                                        *step += 1;
                                    });
                                } else {
                                    // Final step - create the installation
                                    create_installation();
                                }
                            },
                            
                            if *current_step.read() == step_titles.len() - 1 {
                                "Create Installation"
                            } else {
                                "Next"
                            }
                        }
                    }
                }
            }
        }
    }
}

// Account management components
#[component]
fn AccountsPage() -> Element {
    let accounts = get_all_accounts();
    let active_account = get_active_account();
    let mut show_login_dialog = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);
    
    rsx! {
        div { class: "accounts-container",
            h1 { "Account Management" }
            
            // Display error if any
            if let Some(error) = &*error_message.read() {
                div { class: "error-notification",
                    div { class: "error-message", "{error}" }
                    button { 
                        class: "error-close",
                        onclick: move |_| error_message.set(None),
                        "×"
                    }
                }
            }
            
            // Current active account
            div { class: "active-account-section",
                h2 { "Current Account" }
                
                if let Some(account) = active_account {
                    div { class: "active-account-card",
                        img {
                            class: "minecraft-avatar",
                            src: "https://minotar.net/avatar/{account.username}/100.png",
                            alt: "Minecraft Avatar"
                        }
                        
                        div { class: "account-info",
                            h3 { "{account.username}" }
                            
                            if let Some(name) = account.display_name {
                                p { class: "display-name", "{name}" }
                            }
                            
                            p { class: "minecraft-uuid", "UUID: {account.uuid}" }
                            
                            if let Some(last_login) = account.last_login {
                                p { class: "last-login", "Last login: {last_login.format(\"%B %d, %Y\")}" }
                            }
                        }
                        
                        button {
                            class: "sign-out-button",
                            onclick: move |_| {
                                match sign_out() {
                                    Ok(_) => {
                                        // Refresh the page to show updated account status
                                    },
                                    Err(e) => {
                                        error_message.set(Some(e));
                                    }
                                }
                            },
                            "Sign Out"
                        }
                    }
                } else {
                    div { class: "no-account-message",
                        p { "You are not currently signed in to any Microsoft account." }
                        
                        button {
                            class: "sign-in-button",
                            onclick: move |_| {
                                show_login_dialog.set(true);
                            },
                            "Sign In with Microsoft"
                        }
                    }
                }
            }
            
            // Other accounts
            if accounts.len() > 1 {
                div { class: "other-accounts-section",
                    h2 { "Other Accounts" }
                    
                    div { class: "accounts-list",
                        for account in accounts {
                            // Skip active account
                            if active_account.as_ref().map_or(false, |active| active.id == account.id) {
                                {continue}
                            }
                            
                            div { class: "account-list-item",
                                img {
                                    class: "minecraft-avatar-small",
                                    src: "https://minotar.net/avatar/{account.username}/50.png",
                                    alt: "Minecraft Avatar"
                                }
                                
                                div { class: "account-list-info",
                                    p { class: "account-username", "{account.username}" }
                                    
                                    if let Some(name) = account.display_name {
                                        p { class: "account-display-name", "{name}" }
                                    }
                                }
                                
                                div { class: "account-actions",
                                    button {
                                        class: "switch-account-button",
                                        onclick: move |_| {
                                            let account_id = account.id.clone();
                                            match switch_account(&account_id) {
                                                Ok(_) => {
                                                    // Refresh the page
                                                },
                                                Err(e) => {
                                                    error_message.set(Some(e));
                                                }
                                            }
                                        },
                                        "Switch"
                                    }
                                    
                                    button {
                                        class: "remove-account-button",
                                        onclick: move |_| {
                                            // Remove account logic
                                        },
                                        "Remove"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Login dialog
            if *show_login_dialog.read() {
                LoginDialog {
                    onclose: move |_| {
                        show_login_dialog.set(false);
                    },
                    onlogin: move |result| {
                        match result {
                            Ok(_) => {
                                show_login_dialog.set(false);
                                // Refresh the page
                            },
                            Err(e) => {
                                error_message.set(Some(e));
                                show_login_dialog.set(false);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct LoginDialogProps {
    onclose: EventHandler<()>,
    onlogin: EventHandler<Result<(), String>>,
}

#[component]
fn LoginDialog(props: LoginDialogProps) -> Element {
    let mut is_logging_in = use_signal(|| false);
    
    // Login function that handles authentication
    let handle_login = move || {
        is_logging_in.set(true);
        
        // Spawn an async task for authentication
        spawn(async move {
            match authenticate().await {
                Ok(_) => {
                    props.onlogin.call(Ok(()));
                },
                Err(e) => {
                    props.onlogin.call(Err(e));
                }
            }
        });
    };
    
    rsx! {
        div { class: "login-dialog-overlay",
            div { class: "login-dialog",
                div { class: "login-dialog-header", 
                    h2 { "Sign in with Microsoft" }
                    
                    if !*is_logging_in.read() {
                        button {
                            class: "close-button",
                            onclick: move |_| {
                                props.onclose.call(());
                            },
                            "×"
                        }
                    }
                }
                
                div { class: "login-dialog-content",
                    if *is_logging_in.read() {
                        div { class: "login-progress",
                            div { class: "login-spinner" }
                            p { "Waiting for Microsoft authentication..." }
                            p { class: "login-hint", "A browser window should have opened. Please complete the login process there." }
                        }
                    } else {
                        div { class: "login-info",
                            p { "You'll be redirected to Microsoft to sign in to your account. This allows Majestic Overhaul to:" }
                            
                            ul { class: "login-permissions",
                                li { "Verify your Minecraft ownership" }
                                li { "Launch Minecraft with your account" }
                                li { "Read your Minecraft username and UUID" }
                            }
                            
                            p { class: "login-note", "Your Microsoft password is never seen or stored by Majestic Overhaul." }
                            
                            button {
                                class: "microsoft-login-button",
                                onclick: move |_| {
                                    handle_login();
                                },
                                
                                // Microsoft logo (simplified)
                                svg {
                                    class: "ms-logo",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    width: "24",
                                    height: "24",
                                    view_box: "0 0 24 24",
                                    fill: "currentColor",
                                    
                                    rect { x: "3", y: "3", width: "8", height: "8", fill: "#f25022" }
                                    rect { x: "13", y: "3", width: "8", height: "8", fill: "#7fba00" }
                                    rect { x: "3", y: "13", width: "8", height: "8", fill: "#00a4ef" }
                                    rect { x: "13", y: "13", width: "8", height: "8", fill: "#ffb900" }
                                }
                                
                                span { "Continue with Microsoft" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Installation management page
#[component]
fn InstallationDetailsPage(installation_id: String) -> Element {
    // Load the installation
    let installation = match crate::installation::load_installation(&installation_id) {
        Ok(installation) => installation,
        Err(_) => {
            // If installation can't be loaded, show an error
            return rsx! {
                div { class: "error-container",
                    h2 { "Installation Not Found" }
                    p { "The requested installation could not be found." }
                    
                    button {
                        class: "back-button",
                        onclick: move |_| {
                            // Navigate back to home
                        },
                        "Back to Home"
                    }
                }
            };
        }
    };
    
    // Installation status signals
    let mut is_installing = use_signal(|| false);
    let mut installation_error = use_signal(|| Option::<String>::None);
    
    // State for modification tracking
    let mut has_changes = use_signal(|| false);
    
    rsx! {
        div { class: "installation-details-container",
            // Header with installation name and version
            div { class: "installation-header",
                h1 { "{installation.name}" }
                span { class: "minecraft-version", "Minecraft {installation.minecraft_version}" }
                
                if installation.update_available {
                    span { class: "update-badge", "Update Available" }
                }
            }
            
            // Main actions
            div { class: "installation-actions",
                // Play button with authentication check
                PlayButton {
                    uuid: installation.id.clone(),
                    disabled: *is_installing.read(),
                    auth_status: None, // Will auto-detect
                    onclick: move |_| {
                        let mut installation_clone = installation.clone();
                        spawn(async move {
                            match installation_clone.launch() {
                                Ok(_) => {},
                                Err(e) => {
                                    installation_error.set(Some(e));
                                }
                            }
                        });
                    }
                }
                
                // Install/Update button if needed
                if !installation.installed || installation.update_available || *has_changes.read() {
                    button {
                        class: "install-update-button",
                        disabled: *is_installing.read(),
                        onclick: move |_| {
                            is_installing.set(true);
                            let mut installation_clone = installation.clone();
                            let http_client = crate::CachedHttpClient::new();
                            
                            spawn(async move {
                                match installation_clone.install_or_update(&http_client).await {
                                    Ok(_) => {
                                        has_changes.set(false);
                                    },
                                    Err(e) => {
                                        installation_error.set(Some(e));
                                    }
                                }
                                is_installing.set(false);
                            });
                        },
                        
                        if !installation.installed {
                            "Install"
                        } else if installation.update_available {
                            "Update"
                        } else {
                            "Apply Changes"
                        }
                    }
                }
            }
            
            // Error display
            if let Some(error) = &*installation_error.read() {
                div { class: "error-notification",
                    div { class: "error-message", "{error}" }
                    button { 
                        class: "error-close",
                        onclick: move |_| installation_error.set(None),
                        "×"
                    }
                }
            }
            
            // Installation details in tabs
            div { class: "installation-tabs",
                // Tab headers
                // ...
                
                // Tab content would go here
                // Features list, performance settings, etc.
            }
        }
    }
}


#[component]
fn StatisticsDisplay() -> Element {
    rsx! {
        div { class: "stats-container",
            div { class: "stat-item",
                span { class: "stat-value", "200+" }
                span { class: "stat-label", "FPS" }
            }
            div { class: "stat-item",
                span { class: "stat-value", "100+" }
                span { class: "stat-label", "MODS" }
            }
            div { class: "stat-item",
                span { class: "stat-value", "20K+" }
                span { class: "stat-label", "DOWNLOADS" }
            }
        }
    }
}

#[component]
fn ProgressView(
    value: i64,
    max: i64,
    status: String,
    title: String
) -> Element {
    let percentage = if max > 0 { (value * 100) / max } else { 0 };
    
    let steps = vec![
        ("prepare", "Prepare"),
        ("download", "Download"),
        ("extract", "Extract"),
        ("configure", "Configure"),
        ("finish", "Finish")
    ];
    
    // Get current step based on progress
    let current_step = if percentage == 0 {
        "prepare"
    } else if percentage < 30 {
        "download"
    } else if percentage < 60 {
        "extract"
    } else if percentage < 90 {
        "configure"
    } else {
        "finish"
    };
    
    // Mark steps as active or completed based on the current progress
    let active_step_index = steps.iter().position(|(id, _)| id == &current_step).unwrap_or(0);
    
    rsx! {
        div { 
            class: "progress-container",
            "data-value": "{value}",
            "data-max": "{max}",
            "data-step": "{current_step}",
            
            div { class: "progress-header",
                h1 { "{title}" }
                div { class: "progress-subtitle", "Installation in progress..." }
            }
            
            div { class: "progress-content",
                // Step indicators
                div { class: "progress-steps",
                    for (index, (step_id, step_label)) in steps.iter().enumerate() {
                        {
                            let step_class = if index < active_step_index {
                                "progress-step completed"
                            } else if index == active_step_index {
                                "progress-step active"
                            } else {
                                "progress-step"
                            };
                            
                            rsx! {
                                div { 
                                    class: "{step_class}",
                                    "data-step-id": "{step_id}",
                                    
                                    div { class: "step-dot" }
                                    div { class: "step-label", "{step_label}" }
                                }
                            }
                        }
                    }
                }
                
                // Progress bar
                div { class: "progress-track",
                    div { 
                        class: "progress-bar", 
                        style: "width: {percentage}%;"
                    }
                }
                
                // Progress details
                div { class: "progress-details",
                    div { class: "progress-percentage", "{percentage}%" }
                }
                
                p { class: "progress-status", "{status}" }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct CreditsProps {
    manifest: super::Manifest,
    enabled: Vec<String>,
    credits: Signal<bool>,
}

#[component]
fn Credits(mut props: CreditsProps) -> Element {
    rsx! {
        div { class: "credits-container",
            div { class: "credits-header",
                h1 { "{props.manifest.subtitle}" }
                button {
                    class: "close-button",
                    onclick: move |evt| {
                        props.credits.set(false);
                        evt.stop_propagation();
                    },
                    "Close"
                }
            }
            div { class: "credits-content",
                div { class: "credits-list",
                    ul {
                        for r#mod in props.manifest.mods {
                            if props.enabled.contains(&r#mod.id) {
                                li { class: "credit-item",
                                    div { class: "credit-name", "{r#mod.name}" }
                                    div { class: "credit-authors",
                                        "by "
                                        for author in &r#mod.authors {
                                            a { href: "{author.link}", class: "credit-author",
                                                if r#mod.authors.last().unwrap() == author {
                                                    {author.name.to_string()}
                                                } else {
                                                    {author.name.to_string() + ", "}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        for shaderpack in props.manifest.shaderpacks {
                            if props.enabled.contains(&shaderpack.id) {
                                li { class: "credit-item",
                                    div { class: "credit-name", "{shaderpack.name}" }
                                    div { class: "credit-authors",
                                        "by "
                                        for author in &shaderpack.authors {
                                            a { href: "{author.link}", class: "credit-author",
                                                if shaderpack.authors.last().unwrap() == author {
                                                    {author.name.to_string()}
                                                } else {
                                                    {author.name.to_string() + ", "}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        for resourcepack in props.manifest.resourcepacks {
                            if props.enabled.contains(&resourcepack.id) {
                                li { class: "credit-item",
                                    div { class: "credit-name", "{resourcepack.name}" }
                                    div { class: "credit-authors",
                                        "by "
                                        for author in &resourcepack.authors {
                                            a { href: "{author.link}", class: "credit-author",
                                                if resourcepack.authors.last().unwrap() == author {
                                                    {author.name.to_string()}
                                                } else {
                                                    {author.name.to_string() + ", "}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        for include in props.manifest.include {
                            if props.enabled.contains(&include.id) && include.authors.is_some() 
                               && include.name.is_some() {
                                li { class: "credit-item",
                                    div { class: "credit-name", "{include.name.as_ref().unwrap()}" }
                                    div { class: "credit-authors",
                                        "by "
                                        for author in &include.authors.as_ref().unwrap() {
                                            a { href: "{author.link}", class: "credit-author",
                                                if include.authors.as_ref().unwrap().last().unwrap() == author {
                                                    {author.name.to_string()}
                                                } else {
                                                    {author.name.to_string() + ", "}
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
    }
}

#[component]
fn PackUninstallButton(launcher: Launcher, pack: PackName) -> Element {
    let mut hidden = use_signal(|| false);
    rsx!(
        li { hidden,
            button {
                class: "uninstall-list-item",
                onclick: move |_| {
                    uninstall(&launcher, &pack.uuid).unwrap();
                    *hidden.write() = true;
                },
                "{pack.name}"
            }
        }
    )
}

#[derive(PartialEq, Props, Clone)]
struct SettingsProps {
    config: Signal<super::Config>,
    settings: Signal<bool>,
    config_path: PathBuf,
    error: Signal<Option<String>>,
    b64_id: String,
}

#[component]
fn Settings(mut props: SettingsProps) -> Element {
    let mut vanilla = None;
    let mut multimc = None;
    let mut prism = None;
    let mut custom = None;
    let launcher = get_launcher(&props.config.read().launcher).unwrap();
    let packs = match get_installed_packs(&launcher) {
        Ok(v) => v,
        Err(err) => {
            *props.error.write() = Some(err.to_string());
            return None;
        }
    };
    match &props.config.read().launcher[..] {
        "vanilla" => vanilla = Some("true"),
        "multimc-MultiMC" => multimc = Some("true"),
        "multimc-PrismLauncher" => prism = Some("true"),
        _ => {}
    }
    if props.config.read().launcher.starts_with("custom") {
        custom = Some("true")
    }

    rsx! {
        div { class: "settings-container",
            h1 { class: "settings-title", "Settings" }
            form {
                id: "settings",
                class: "settings-form",
                onsubmit: move |event| {
                    props
                        .config
                        .write()
                        .launcher = event.data.values()["launcher-select"].as_value();
                    if let Err(e) = std::fs::write(
                        &props.config_path,
                        serde_json::to_vec(&*props.config.read()).unwrap(),
                    ) {
                        props.error.set(Some(format!("{:#?}", e) + " (Failed to write config!)"));
                    }
                    props.settings.set(false);
                },
                
                div { class: "setting-group",
                    label { class: "setting-label", "Launcher:" }
                    select {
                        name: "launcher-select",
                        id: "launcher-select",
                        form: "settings",
                        class: "setting-select",
                        if super::get_minecraft_folder().is_dir() {
                            option { value: "vanilla", selected: vanilla, "Vanilla" }
                        }
                        if super::get_multimc_folder("MultiMC").is_ok() {
                            option { value: "multimc-MultiMC", selected: multimc, "MultiMC" }
                        }
                        if super::get_multimc_folder("PrismLauncher").is_ok() {
                            option {
                                value: "multimc-PrismLauncher",
                                selected: prism,
                                "Prism Launcher"
                            }
                        }
                        if custom.is_some() {
                            option {
                                value: "{props.config.read().launcher}",
                                selected: custom,
                                "Custom MultiMC"
                            }
                        }
                    }
                }
                
                CustomMultiMCButton {
                    config: props.config,
                    config_path: props.config_path.clone(),
                    error: props.error,
                    b64_id: props.b64_id.clone()
                }
                
                div { class: "settings-buttons",
                    input {
                        r#type: "submit",
                        value: "Save",
                        class: "primary-button",
                        id: "save"
                    }
                    
                    button {
                        class: "secondary-button",
                        r#type: "button",
                        disabled: packs.is_empty(),
                        onclick: move |evt| {
                            let mut modal = use_context::<ModalContext>();
                            modal
                                .open(
                                    "Select modpack to uninstall",
                                    rsx! {
                                        div { class: "uninstall-list-container",
                                            ul { class: "uninstall-list",
                                                for pack in packs.clone() {
                                                    PackUninstallButton { launcher: launcher.clone(), pack }
                                                }
                                            }
                                        }
                                    },
                                    false,
                                    Some(|_| {}),
                                );
                            evt.stop_propagation();
                        },
                        "Uninstall"
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct LauncherProps {
    config: Signal<super::Config>,
    config_path: PathBuf,
    error: Signal<Option<String>>,
    b64_id: String,
}

#[component]
fn InstallButton(
    label: String,
    disabled: bool,
    onclick: EventHandler<MouseEvent>,
    state: Option<String>  // "ready", "processing", "success", "updating", "modified"
) -> Element {
    let button_state = state.unwrap_or_else(|| "ready".to_string());
    
    rsx! {
        div { class: "install-button-container",
            div { 
                class: "button-scale-wrapper",
                style: "animation: button-scale-pulse 3s infinite alternate, button-breathe 4s infinite ease-in-out;",
                button {
                    class: "main-install-button",
                    disabled: disabled,
                    "data-state": "{button_state}",
                    onclick: move |evt| onclick.call(evt),
                    
                    span { class: "button-text", "{label}" }
                    div { class: "button-progress" }
                }
            }
        }
    }
}

#[component]
fn Launcher(mut props: LauncherProps) -> Element {
    let mut vanilla = None;
    let mut multimc = None;
    let mut prism = None;
    match &props.config.read().launcher[..] {
        "vanilla" => vanilla = Some("true"),
        "multimc-MultiMC" => multimc = Some("true"),
        "multimc-PrismLauncher" => prism = Some("true"),
        _ => {}
    }
    let has_supported_launcher = super::get_minecraft_folder().is_dir()
        || super::get_multimc_folder("MultiMC").is_ok()
        || super::get_multimc_folder("PrismLauncher").is_ok();
        
    if !has_supported_launcher {
        rsx!(NoLauncherFound {
            config: props.config,
            config_path: props.config_path,
            error: props.error,
            b64_id: props.b64_id.clone()
        })
    } else {
        rsx! {
            div { class: "launcher-container",
                h1 { class: "launcher-title", "Select Launcher" }
                form {
                    id: "launcher-form",
                    class: "launcher-form",
                    onsubmit: move |event| {
                        props
                            .config
                            .write()
                            .launcher = event.data.values()["launcher-select"].as_value();
                        props.config.write().first_launch = Some(false);
                        if let Err(e) = std::fs::write(
                            &props.config_path,
                            serde_json::to_vec(&*props.config.read()).unwrap(),
                        ) {
                            props.error.set(Some(format!("{:#?}", e) + " (Failed to write config!)"));
                        }
                    },
                    
                    div { class: "setting-group",
                        label { class: "setting-label", "Launcher:" }
                        select {
                            name: "launcher-select",
                            id: "launcher-select",
                            form: "launcher-form",
                            class: "setting-select",
                            if super::get_minecraft_folder().is_dir() {
                                option { value: "vanilla", selected: vanilla, "Vanilla" }
                            }
                            if super::get_multimc_folder("MultiMC").is_ok() {
                                option {
                                    value: "multimc-MultiMC",
                                    selected: multimc,
                                    "MultiMC"
                                }
                            }
                            if super::get_multimc_folder("PrismLauncher").is_ok() {
                                option {
                                    value: "multimc-PrismLauncher",
                                    selected: prism,
                                    "Prism Launcher"
                                }
                            }
                        }
                    }
                    
                    CustomMultiMCButton {
                        config: props.config,
                        config_path: props.config_path.clone(),
                        error: props.error,
                        b64_id: props.b64_id.clone()
                    }
                    
                    input {
                        r#type: "submit",
                        value: "Continue",
                        class: "primary-button",
                        id: "continue-button"
                    }
                }
            }
        }
    }
}

#[component]
fn CustomMultiMCButton(mut props: LauncherProps) -> Element {
    let custom_multimc = move |_evt| {
        let directory_dialog = rfd::FileDialog::new()
            .set_title("Pick root directory of desired MultiMC based launcher.")
            .set_directory(get_app_data());
        let directory = directory_dialog.pick_folder();
        if let Some(path) = directory {
            if !path.join("instances").is_dir() {
                return;
            }
            let path = path.to_str();
            if path.is_none() {
                props
                    .error
                    .set(Some(String::from("Could not get path to directory!")));
            }
            props.config.write().launcher = format!("custom-{}", path.unwrap());
            props.config.write().first_launch = Some(false);
            if let Err(e) = std::fs::write(
                &props.config_path,
                serde_json::to_vec(&*props.config.read()).unwrap(),
            ) {
                props
                    .error
                    .set(Some(format!("{:#?}", e) + " (Failed to write config!)"));
            }
        }
    };
    
    rsx!(
        button {
            class: "secondary-button custom-multimc-button",
            onclick: custom_multimc,
            r#type: "button",
            "Use custom MultiMC directory"
        }
    )
}

#[component]
fn NoLauncherFound(props: LauncherProps) -> Element {
    rsx! {
        div { class: "no-launcher-container",
            h1 { class: "no-launcher-title", "No supported launcher found!" }
            div { class: "no-launcher-message",
                p {
                    "Only Prism Launcher, MultiMC and the vanilla launcher are supported by default, other MultiMC launchers can be added using the button below."
                }
                p {
                    "If you have any of these installed then please make sure you are on the latest version of the installer, if you are, open a thread in #📂modpack-issues on the discord. Please make sure your thread contains the following information: Launcher your having issues with, directory of the launcher and your OS."
                }
            }
            CustomMultiMCButton {
                config: props.config,
                config_path: props.config_path,
                error: props.error,
                b64_id: props.b64_id.clone()
            }
        }
    }
}

// Feature Card component to display features in card format
#[derive(PartialEq, Props, Clone)]
struct FeatureCardProps {
    feature: super::Feature,
    enabled: bool,
    on_toggle: EventHandler<FormEvent>,
}

#[component]
fn FeatureCard(props: FeatureCardProps) -> Element {
    let enabled = props.enabled;
    let feature_id = props.feature.id.clone();
    
    rsx! {
        div { 
            class: if enabled { "feature-card feature-enabled" } else { "feature-card feature-disabled" },
            div { class: "feature-card-header",
                h3 { class: "feature-card-title", "{props.feature.name}" }
                
                // Toggle button with properly connected event handler - moved to header
                label {
                    class: if enabled { "feature-toggle-button enabled" } else { "feature-toggle-button disabled" },
                    input {
                        r#type: "checkbox",
                        name: "{feature_id}",
                        checked: if enabled { Some("true") } else { None },
                        onchange: move |evt| props.on_toggle.call(evt),
                        style: "display: none;"
                    }
                    if enabled { "ON" } else { "OFF" }
                }
            }
            
            // Render description if available, but only if it exists
            if let Some(description) = &props.feature.description {
                if !description.is_empty() {
                    div { class: "feature-card-description", "{description}" }
                }
            }
        }
    }
}



fn feature_change(
    local_features: Signal<Option<Vec<String>>>,
    mut modify: Signal<bool>,
    evt: FormEvent,
    feat: &super::Feature,
    mut modify_count: Signal<i32>,
    mut enabled_features: Signal<Vec<String>>,
) {
    // Extract values first
    let enabled = match &*evt.data.value() {
        "true" => true,
        "false" => false,
        _ => panic!("Invalid bool from feature"),
    };
    
    debug!("Feature toggle changed: {} -> {}", feat.id, enabled);
    
    // Copy values we need for comparison
    let current_features = enabled_features.read().clone();
    let contains_feature = current_features.contains(&feat.id);
    let current_count = *modify_count.read();
    
    // Only update if necessary
    if enabled != contains_feature {
        debug!("Updating feature state for {}: {} -> {}", feat.id, contains_feature, enabled);
        enabled_features.with_mut(|x| {
            if enabled && !x.contains(&feat.id) {
                x.push(feat.id.clone());
            } else if !enabled {
                x.retain(|item| item != &feat.id);
            }
        });
    }
    
    // Handle modify signals in a separate step
    if let Some(local_feat) = local_features.read().as_ref() {
        let modify_res = local_feat.contains(&feat.id) != enabled;
        
        // Schedule these operations separately to avoid infinite loop warnings
        if current_count <= 1 {
            modify.set(modify_res);
        }
        
        if modify_res {
            modify_count.with_mut(|x| *x += 1);
        } else {
            modify_count.with_mut(|x| *x -= 1);
        }
    }
}

// Update the init_branch function
async fn init_branch(source: String, branch: String, launcher: Launcher, mut pages: Signal<BTreeMap<usize, TabInfo>>) -> Result<(), String> {
    debug!("Initializing modpack from source: {}, branch: {}", source, branch);
    let profile = crate::init(source.to_owned(), branch.to_owned(), launcher).await?;

    // Process manifest data for tab information
    debug!("Processing manifest tab information:");
    debug!("  subtitle: {}", profile.manifest.subtitle);
    debug!("  description length: {}", profile.manifest.description.len());

    let tab_group = if let Some(tab_group) = profile.manifest.tab_group {
        debug!("  tab_group: {}", tab_group);
        tab_group
    } else {
        debug!("  tab_group: None, defaulting to 0");
        0
    };

    // Check if this profile already exists in the tab group
    let profile_exists = pages.read().get(&tab_group)
        .map_or(false, |tab_info| tab_info.modpacks.iter()
            .any(|p| p.modpack_branch == profile.modpack_branch && p.modpack_source == profile.modpack_source));
            
    if profile_exists {
        debug!("Profile already exists in tab_group {}, skipping", tab_group);
        return Ok(());
    }

    let tab_created = pages.read().contains_key(&tab_group);
    
    // Create the tab if it doesn't exist
    if !tab_created {
        let tab_title = if let Some(ref tab_title) = profile.manifest.tab_title {
            debug!("  tab_title: {}", tab_title);
            tab_title.clone()
        } else {
            debug!("  tab_title: None, using subtitle");
            profile.manifest.subtitle.clone()
        };

        let tab_color = if let Some(ref tab_color) = profile.manifest.tab_color {
            debug!("  tab_color: {}", tab_color);
            tab_color.clone()
        } else {
            debug!("  tab_color: None, defaulting to '#320625'");
            String::from("#320625")
        };

        let tab_background = if let Some(ref tab_background) = profile.manifest.tab_background {
            debug!("  tab_background: {}", tab_background);
            tab_background.clone()
        } else {
            let default_bg = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png";
            debug!("  tab_background: None, defaulting to '{}'", default_bg);
            String::from(default_bg)
        };

        // Use a consistent background for settings - home background
        let settings_background = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png".to_string();

        // No longer storing font variables in TabInfo
        let tab_info = TabInfo {
            color: tab_color,
            title: tab_title,
            background: tab_background,
            settings_background,
            // Remove these fields as we're now using constants
            // primary_font: consistent_font.clone(),
            // secondary_font: consistent_font.clone(),
            modpacks: vec![profile.clone()], // Add the profile immediately
        };
        
        pages.write().insert(tab_group, tab_info);
        debug!("Created tab_group {} with profile {}", tab_group, branch);
    } else {
        // Add the profile to an existing tab
        pages.write().entry(tab_group).and_modify(|tab_info| {
            tab_info.modpacks.push(profile.clone());
            debug!("Added profile {} to existing tab_group {}", branch, tab_group);
        });
    }

    Ok(())
}

#[derive(PartialEq, Props, Clone)]
struct VersionProps {
    installer_profile: InstallerProfile,
    error: Signal<Option<String>>,
    current_page: usize,
    tab_group: usize,
}

#[component]
fn Version(mut props: VersionProps) -> Element {
    let installer_profile = props.installer_profile.clone();
    
    // Add explicit debugging for initial state
    debug!("INITIAL STATE: installed={}, update_available={}", 
           installer_profile.installed, installer_profile.update_available);
    
    // Force reactivity with explicit signal declarations and consistent usage
    let mut installing = use_signal(|| false);
    let mut progress_status = use_signal(|| "".to_string());
    let mut install_progress = use_signal(|| 0);
    let mut modify = use_signal(|| false);
    let mut modify_count = use_signal(|| 0);
    let mut credits = use_signal(|| false);
    let mut expanded_features = use_signal(|| false);
    
    // Convert these to mutable signals to ensure their changes trigger rerendering
    let mut installed = use_signal(|| installer_profile.installed);
    let mut update_available = use_signal(|| installer_profile.update_available);
    let mut install_item_amount = use_signal(|| 0);

    // Create a debug signal to force refreshes when needed
    let mut debug_counter = use_signal(|| 0);
    
    // IMPORTANT: Store the features collection in a signal to solve lifetime issues
    let features = use_signal(|| installer_profile.manifest.features.clone());
    
    // Clone the UUID right away to avoid ownership issues
    let uuid = installer_profile.manifest.uuid.clone();
    
    // Add debugging to watch for signal changes
    use_effect(move || {
        debug!("SIGNAL UPDATE: installed={}, update_available={}, modify={}, credits={}, debug_counter={}",
               *installed.read(), *update_available.read(), *modify.read(), *credits.read(), *debug_counter.read());
    });

    // Use signal for enabled_features with cleaner initialization
    let mut enabled_features = use_signal(|| {
        let mut feature_list = vec!["default".to_string()];
        
        if installer_profile.installed && installer_profile.local_manifest.is_some() {
            feature_list = installer_profile.local_manifest.as_ref().unwrap().enabled_features.clone();
        } else {
            // Add default features
            for feat in &installer_profile.manifest.features {
                if feat.default {
                    feature_list.push(feat.id.clone());
                }
            }
        }

        debug!("Initialized enabled_features: {:?}", feature_list);
        feature_list
    });
    
    // Clone local_manifest to prevent ownership issues
    let mut local_features = use_signal(|| {
        if let Some(ref manifest) = installer_profile.local_manifest {
            Some(manifest.enabled_features.clone())
        } else {
            None
        }
    });
    
    // Calculate how many features to show in first row - default to 3
    let first_row_count = 3;
    
    // Feature toggle handler function
    let mut handle_feature_toggle = move |feat: super::Feature, evt: FormEvent| {
        // Extract form value
        let enabled = match &*evt.data.value() {
            "true" => true,
            "false" => false,
            _ => panic!("Invalid bool from feature"),
        };
        
        debug!("Feature toggle changed: {} -> {}", feat.id, enabled);
        
        // Update enabled_features
        enabled_features.with_mut(|feature_list| {
            if enabled {
                if !feature_list.contains(&feat.id) {
                    feature_list.push(feat.id.clone());
                    debug!("Added feature: {}", feat.id);
                }
            } else {
                feature_list.retain(|id| id != &feat.id);
                debug!("Removed feature: {}", feat.id);
            }
        });
        
        // Handle modify flag
        if let Some(local_feat) = local_features.read().as_ref() {
            let was_enabled = local_feat.contains(&feat.id);
            let is_modified = was_enabled != enabled;
            
            debug!("Feature modified check: was_enabled={}, new_state={}, is_modified={}", 
                   was_enabled, enabled, is_modified);
            
            if is_modified {
                modify_count.with_mut(|x| *x += 1);
                if *modify_count.read() > 0 {
                    modify.set(true);
                    debug!("SET MODIFY FLAG: true");
                }
            } else {
                modify_count.with_mut(|x| *x -= 1);
                if *modify_count.read() <= 0 {
                    modify.set(false);
                    debug!("SET MODIFY FLAG: false");
                }
            }
        }
        
        // Force refresh
        debug_counter.with_mut(|x| *x += 1);
    };
    
    // Installation/update submit handler
    let movable_profile = installer_profile.clone();
    let on_submit = move |_| {
        // Calculate total items to process for progress tracking
        *install_item_amount.write() = movable_profile.manifest.mods.len()
            + movable_profile.manifest.resourcepacks.len()
            + movable_profile.manifest.shaderpacks.len()
            + movable_profile.manifest.include.len();
        
        let movable_profile = movable_profile.clone();
        let movable_profile2 = movable_profile.clone();
        
        async move {
            let install = move |canceled| {
                let mut installer_profile = movable_profile.clone();
                spawn(async move {
                    if canceled {
                        return;
                    }
                    installing.set(true);
                    installer_profile.enabled_features = enabled_features.read().clone();
                    installer_profile.manifest.enabled_features = enabled_features.read().clone();
                    local_features.set(Some(enabled_features.read().clone()));

                    if !*installed.read() {
                        progress_status.set("Installing".to_string());
                        match crate::install(&installer_profile, move || {
                            install_progress.with_mut(|x| *x += 1);
                        })
                        .await
                        {
                            Ok(_) => {
                                installed.set(true);
                                debug!("SET INSTALLED: true");
                                
                                let _ = isahc::post(
                                    "https://tracking.commander07.workers.dev/track",
                                    format!(
                                        "{{
                                    \"projectId\": \"55db8403a4f24f3aa5afd33fd1962888\",
                                    \"dataSourceId\": \"{}\",
                                    \"userAction\": \"update\",
                                    \"additionalData\": {{
                                        \"old_version\": \"{}\",
                                        \"new_version\": \"{}\"
                                    }}
                                }}",
                                        installer_profile.manifest.uuid,
                                        installer_profile.local_manifest.unwrap().modpack_version,
                                        installer_profile.manifest.modpack_version
                                    ),
                                );
                            }
                            Err(e) => {
                                props.error.set(Some(
                                    format!("{:#?}", e) + " (Failed to update modpack!)",
                                ));
                                installing.set(false);
                                return;
                            }
                        }
                        update_available.set(false);
                        debug!("SET UPDATE_AVAILABLE: false");
                    } else if *modify.read() {
                        progress_status.set("Modifying".to_string());
                        match super::update(&installer_profile, move || {
                            install_progress.with_mut(|x| *x += 1);
                        })
                        .await
                        {
                            Ok(_) => {
                                let _ = isahc::post(
                                    "https://tracking.commander07.workers.dev/track",
                                    format!(
                                        "{{
                                    \"projectId\": \"55db8403a4f24f3aa5afd33fd1962888\",
                                    \"dataSourceId\": \"{}\",
                                    \"userAction\": \"modify\",
                                    \"additionalData\": {{
                                        \"features\": {:?}
                                    }}
                                }}",
                                        installer_profile.manifest.uuid,
                                        installer_profile.manifest.enabled_features
                                    ),
                                );
                            }
                            Err(e) => {
                                props.error.set(Some(
                                    format!("{:#?}", e) + " (Failed to modify modpack!)",
                                ));
                                installing.set(false);
                                return;
                            }
                        }
                        modify.set(false);
                        debug!("RESET MODIFY: false");
                        modify_count.set(0);
                        update_available.set(false);
                        debug!("SET UPDATE_AVAILABLE: false");
                    }
                    installing.set(false);
                    
                    // Force refresh
                    debug_counter.with_mut(|x| *x += 1);
                });
            };

            if let Some(contents) = movable_profile2.manifest.popup_contents {
                use_context::<ModalContext>().open(
                    movable_profile2.manifest.popup_title.unwrap_or_default(),
                    rsx!(div {
                        dangerous_inner_html: "{contents}",
                    }),
                    true,
                    Some(install),
                )
            } else {
                install(false);
            }
        }
    };

    // Button label based on state
    let button_label = if !*installed.read() {
        debug!("Button state: Install");
        "Install"
    } else if *update_available.read() {
        debug!("Button state: Update");
        "Update"
    } else if *modify.read() {
        debug!("Button state: Modify");
        "Modify"
    } else {
        debug!("Button state: Modify (default)");
        "Modify"
    };
    
    // Button disable logic
    let install_disable = *installed.read() && !*update_available.read() && !*modify.read();
    debug!("Button disabled: {}", install_disable);
    
    rsx! {
        if *installing.read() {
            ProgressView {
                value: *install_progress.read(),
                max: *install_item_amount.read() as i64,
                title: installer_profile.manifest.subtitle.clone(),
                status: progress_status.read().clone()
            }
        } else if *credits.read() {
            Credits {
                manifest: installer_profile.manifest.clone(),
                enabled: enabled_features.read().clone(),
                credits
            }
        } else {
            div { class: "version-container",
                "<!-- debug counter: {*debug_counter.read()} -->",
                
                form { onsubmit: on_submit,
                    // Header section with title and subtitle
                    div { class: "content-header",
                        h1 { "{installer_profile.manifest.subtitle}" }
                    }
                    
                    // Description section
                    div { class: "content-description",
                        dangerous_inner_html: "{installer_profile.manifest.description}",
                        
                        // Credits link
                        div { class: "credits-link-container", style: "text-align: center; margin: 15px 0;",
                            a {
                                class: "credits-button",
                                onclick: move |evt| {
                                    debug!("Credits clicked");
                                    credits.set(true);
                                    debug!("SET CREDITS: true");
                                    evt.stop_propagation();
                                },
                                "VIEW CREDITS"
                            }
                        }
                    }
                    
                    // Expandable Features Section
                    div { class: "features-section",
                        h2 { class: "features-heading", "OPTIONAL FEATURES" }
                        
                        div { class: "feature-cards-container",
                            {
                                // Filter features inside the RSX block
                                let features_list = features.read();
                                let visible_features: Vec<_> = features_list.iter()
                                    .filter(|f| !f.hidden)
                                    .collect();
                                
                                // Calculate whether to show expand button
                                let show_expand_button = visible_features.len() > first_row_count;
                                
                                rsx! {
                                    // First row of features (always shown)
                                    for feat in visible_features.iter().take(first_row_count) {
                                        {
                                            let is_enabled = enabled_features.read().contains(&feat.id);
                                            let feat_clone = (*feat).clone();
                                            
                                            rsx! {
                                                div { 
                                                    class: if is_enabled { "feature-card feature-enabled" } else { "feature-card feature-disabled" },
                                                    div { class: "feature-card-header",
                                                        h3 { class: "feature-card-title", "{feat.name}" }
                                                    }
                                                    
                                                    if let Some(description) = &feat.description {
                                                        div { class: "feature-card-description", "{description}" }
                                                    }
                                                    
                                                    label {
                                                        class: if is_enabled { "feature-toggle-button enabled" } else { "feature-toggle-button disabled" },
                                                        input {
                                                            r#type: "checkbox",
                                                            name: "{feat.id}",
                                                            checked: if is_enabled { Some("true") } else { None },
                                                            onchange: move |evt| handle_feature_toggle(feat_clone.clone(), evt),
                                                            style: "display: none;"
                                                        }
                                                        if is_enabled { "Enabled" } else { "Disabled" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Additional features (shown only when expanded)
                                    if *expanded_features.read() {
                                        for feat in visible_features.iter().skip(first_row_count) {
                                            {
                                                let is_enabled = enabled_features.read().contains(&feat.id);
                                                let feat_clone = (*feat).clone();
                                                
                                                rsx! {
                                                    div { 
                                                        class: if is_enabled { "feature-card feature-enabled" } else { "feature-card feature-disabled" },
                                                        div { class: "feature-card-header",
                                                            h3 { class: "feature-card-title", "{feat.name}" }
                                                        }
                                                        
                                                        if let Some(description) = &feat.description {
                                                            div { class: "feature-card-description", "{description}" }
                                                        }
                                                        
                                                        label {
                                                            class: if is_enabled { "feature-toggle-button enabled" } else { "feature-toggle-button disabled" },
                                                            input {
                                                                r#type: "checkbox",
                                                                name: "{feat.id}",
                                                                checked: if is_enabled { Some("true") } else { None },
                                                                onchange: move |evt| handle_feature_toggle(feat_clone.clone(), evt),
                                                                style: "display: none;"
                                                            }
                                                            if is_enabled { "Enabled" } else { "Disabled" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Only show expand button if needed
                                    if show_expand_button {
                                        div { class: "features-expand-container",
                                            button {
                                                class: "features-expand-button",
                                                onclick: move |_| {
                                                    let current_state = *expanded_features.read();
                                                    expanded_features.set(!current_state);
                                                    debug!("Toggled expanded features: {}", !current_state);
                                                },
                                                if *expanded_features.read() {
                                                    "Collapse Features"
                                                } else {
                                                    {format!("Show {} More Features", visible_features.len() - first_row_count)}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Install/Update/Modify button at the bottom with explicit label
                    div { class: "install-button-container",
                        div { class: "button-scale-wrapper",
                            button {
                                class: "main-install-button",
                                disabled: install_disable,
                                "{button_label}"
                            }
                        }
                    }
                    
                    // Add Play button only when installed
                    if *installed.read() {
                        {
                            let uuid_str = uuid.clone(); // Clone outside
                            let err_signal = props.error.clone(); // Use props.error
                            
                            rsx! {
                                PlayButton {
                                    uuid: uuid_str.clone(), // Clone again for component
                                    disabled: false,
                                    auth_status: None,
                                    onclick: move |_| {
                                        let uuid_for_handler = uuid_str.clone(); // Clone inside closure
                                        handle_play_click(uuid_for_handler, &err_signal);
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

/// New header component with tabs - updated to display tab groups 1-3 in main row
#[component]
fn AppHeader(
    page: Signal<usize>, 
    pages: Signal<BTreeMap<usize, TabInfo>>,
    settings: Signal<bool>,
    logo_url: Option<String>
) -> Element {
    // Debug what tabs we have available
    debug!("AppHeader: rendering with {} tabs", pages().len());
    for (index, info) in pages().iter() {
        debug!("  Tab {}: title={}", index, info.title);
    }
    
    // We need to collect the info we need from pages() into local structures
    // to avoid lifetime issues
    let mut main_tab_indices = vec![];
    let mut main_tab_titles = vec![];
    let mut dropdown_tab_indices = vec![];
    let mut dropdown_tab_titles = vec![];
    
    // Separate tab groups into main tabs (1, 2, 3) and dropdown tabs (0, 4+)
    for (index, info) in pages().iter() {
        if *index >= 1 && *index <= 3 {
            main_tab_indices.push(*index);
            main_tab_titles.push(info.title.clone());
        } else {
            dropdown_tab_indices.push(*index);
            dropdown_tab_titles.push(info.title.clone());
        }
    }
    
    let has_dropdown = !dropdown_tab_indices.is_empty();
    let any_dropdown_active = dropdown_tab_indices.iter().any(|idx| page() == *idx);
  
    rsx!(
        header { class: "app-header",
            // Logo (if available) serves as home button
            if let Some(url) = logo_url {
                img { 
                    class: "app-logo", 
                    src: "{url}", 
                    alt: "Logo",
                    onclick: move |_| {
                        page.set(HOME_PAGE);
                        debug!("Navigating to home page via logo");
                    },
                    style: "cursor: pointer;"
                }
            }
            
            h1 { 
                class: "app-title", 
                onclick: move |_| {
                    page.set(HOME_PAGE);
                    debug!("Navigating to home page via title");
                },
                style: "cursor: pointer;",
                "OVERHAUL INSTALLER" 
            }
            
            // Tabs from pages - show only if we have pages
            div { class: "header-tabs",
                // Home tab
                button {
                    class: if page() == HOME_PAGE { "header-tab-button active" } else { "header-tab-button" },
                    onclick: move |_| {
                        page.set(HOME_PAGE);
                        debug!("Navigating to home page via tab");
                    },
                    "Home"
                }

                // Main tabs (1, 2, 3)
                {
                    main_tab_indices.iter().enumerate().map(|(i, &index)| {
                        let title = main_tab_titles[i].clone();
                        rsx!(
                            button {
                                class: if page() == index { "header-tab-button active" } else { "header-tab-button" },
                                onclick: move |_| {
                                    debug!("TAB CLICK: Changing page from {} to {}", page(), index);
                                    // CRITICAL FIX: Use write() for more direct access
                                    page.write().clone_from(&index);
                                    debug!("TAB CLICK RESULT: Page is now {}", page());
                                },
                                "{title}"
                            }
                        )
                    })
                }
                
                // Dropdown for remaining tabs - placed outside the flow to avoid affecting scrolling
                if has_dropdown {
                    div { 
                        class: "dropdown",
                        button {
                            class: if any_dropdown_active { 
                                "header-tab-button active" 
                            } else { 
                                "header-tab-button" 
                            },
                            "More ▼"
                        }
                        div { 
                            class: "dropdown-content",
                            {
                                dropdown_tab_indices.iter().enumerate().map(|(i, &index)| {
                                    let title = dropdown_tab_titles[i].clone();
                                    rsx!(
                                        button {
                                            class: if page() == index { "dropdown-item active" } else { "dropdown-item" },
                                            onclick: move |_| {
                                                page.set(index);
                                                debug!("Switching to dropdown tab {}: {}", index, title);
                                            },
                                            "{title}"
                                        }
                                    )
                                })
                            }
                        }
                    }
                } else if pages().is_empty() {
                    // If no tabs, show a message for debugging purposes
                    span { style: "color: #888; font-style: italic;", "Loading tabs..." }
                }
            }
            
            // Settings button
            button {
                class: "settings-button",
                onclick: move |_| {
                    settings.set(true);
                    debug!("Opening settings");
                },
                "Settings"
            }
        }
    )
}

#[derive(Clone)]
pub(crate) struct AppProps {
    pub branches: Vec<super::GithubBranch>,
    pub modpack_source: String,
    pub config: super::Config,
    pub config_path: PathBuf,
    // We can optionally add the installations field
    // but will need to update where it's used
}

// Replace the entire app() function with this properly structured version

pub(crate) fn app() -> Element {
    let props = use_context::<AppProps>();
    let css = include_str!("assets/style.css");
    let branches = props.branches.clone();
    let config = use_signal(|| props.config);
    let settings = use_signal(|| false);
    let mut err: Signal<Option<String>> = use_signal(|| None);
    let page = use_signal(|| HOME_PAGE);  // Initially set to HOME_PAGE
    let mut pages = use_signal(BTreeMap::<usize, TabInfo>::new);
    

    // DIAGNOSTIC: Print branches available
    debug!("DIAGNOSTIC: Available branches: {}", branches.len());
    for branch in &branches {
        debug!("  - Branch: {}", branch.name);
    }

    // DIAGNOSTIC: Add direct modification of the page signal to verify reactivity
    use_effect(move || {
        debug!("DIAGNOSTIC: Current page value: {}", page());
        debug!("DIAGNOSTIC: HOME_PAGE value: {}", HOME_PAGE);

        // Debug the pages map
        debug!("DIAGNOSTIC: Pages map contains {} entries", pages().len());
        for (key, info) in pages().iter() {
            debug!("  - Tab group {}: {} with {} modpacks", 
                   key, info.title, info.modpacks.len());
            
            // List modpacks in each tab group
            for (i, profile) in info.modpacks.iter().enumerate() {
                debug!("    * Modpack {}: {}", i, profile.manifest.subtitle);
            }
        }
    });

    let cfg = config.with(|cfg| cfg.clone());
    let launcher = match super::get_launcher(&cfg.launcher) {
        Ok(val) => {
            debug!("Successfully loaded launcher: {}", cfg.launcher);
            Some(val)
        },
        Err(e) => {
            error!("Failed to load launcher: {} - {}", cfg.launcher, e);
            None
        },
    };

    // Debug logging for branches
    debug!("Total branches: {}", branches.len());
    for branch in &branches {
        debug!("Branch: {}", branch.name);
    }

    // Modified resource to process branches
    let packs: Resource<Vec<(usize, InstallerProfile)>> = {
        let source = props.modpack_source.clone();
        let branches = branches.clone();
        let launcher = launcher.clone();
        use_resource(move || {
            let source = source.clone();
            let branches = branches.clone();
            let launcher = launcher.clone();
            async move {
                let mut results = Vec::new();
                if let Some(launcher) = launcher {
                    for branch in &branches {
                        match crate::init(source.clone(), branch.name.clone(), launcher.clone()).await {
                            Ok(profile) => {
                                let tab_group = profile.manifest.tab_group.unwrap_or(0);
                                results.push((tab_group, profile));
                                debug!("Processed branch: {} in tab group {}", branch.name, tab_group);
                            }
                            Err(e) => {
                                error!("Failed to initialize branch {}: {}", branch.name, e);
                            }
                        }
                    }
                }
                results
            }
        })
    };

    // Effect to build pages map when branches are processed
    use_effect(move || {
        if let Some(processed_branches) = packs.read().as_ref() {
            debug!("Building pages map from {} processed branches", processed_branches.len());
            
            let mut new_pages = BTreeMap::<usize, TabInfo>::new();
            for (tab_group, profile) in processed_branches {
                let tab_title = profile.manifest.tab_title.clone().unwrap_or_else(|| profile.manifest.subtitle.clone());
                let tab_color = profile.manifest.tab_color.clone().unwrap_or_else(|| String::from("#320625"));
                let tab_background = profile.manifest.tab_background.clone().unwrap_or_else(|| {
                    String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png")
                });
                let settings_background = profile.manifest.settings_background.clone().unwrap_or_else(|| tab_background.clone());
                
                // No longer including font fields
                new_pages.entry(*tab_group).or_insert(TabInfo {
                    color: tab_color,
                    title: tab_title,
                    background: tab_background,
                    settings_background,
                    // Remove these fields
                    // primary_font,
                    // secondary_font,
                    modpacks: vec![profile.clone()],
                });
            }
            
            pages.set(new_pages);
            debug!("Updated pages map with {} tabs", pages().len());
        }
    });

    let css_content = {
        let default_color = "#320625".to_string();
        let default_bg = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png".to_string();
        
        let bg_color = match pages().get(&page()) {
            Some(x) => x.color.clone(),
            None => default_color,
        };
        
        let bg_image = match pages().get(&page()) {
            Some(x) => {
                if settings() {
                    x.settings_background.clone()
                } else {
                    x.background.clone()
                }
            },
            None => default_bg,
        };
        
        // Use constants instead of TabInfo properties
        debug!("Updating CSS with: color={}, bg_image={}", bg_color, bg_image);
            
        // Improved dropdown menu CSS with better hover behavior and font consistency
        let _dropdown_css = "
        /* Dropdown styles */
        .dropdown { 
            position: relative; 
            display: inline-block; 
        }

        /* Position the dropdown content */
        .dropdown-content {
            display: none;
            position: absolute;
            top: 100%;
            left: 0;
            background-color: rgba(0, 0, 0, 0.9);
            min-width: 200px;
            box-shadow: 0 8px 16px rgba(0, 0, 0, 0.6);
            z-index: 1000;
            border-radius: 4px;
            overflow: hidden;
            margin-top: 5px;
            max-height: 400px;
            overflow-y: auto;
            border: 1px solid rgba(255, 255, 255, 0.1);
        }

        /* Show dropdown on hover with increased target area */
        .dropdown:hover .dropdown-content,
        .dropdown-content:hover {
            display: block;
        }

        /* Add a pseudo-element to create an invisible connection between the button and dropdown */
        .dropdown::after {
            content: '';
            position: absolute;
            height: 10px;
            width: 100%;
            left: 0;
            top: 100%;
            display: none;
        }

        .dropdown:hover::after {
            display: block;
        }

        .dropdown-item {
            display: block;
            width: 100%;
            padding: 10px 15px;
            text-align: left;
            background-color: transparent;
            border: none;
            /* Explicitly use the PRIMARY_FONT */
            font-family: \\\"PRIMARY_FONT\\\";
            font-size: 0.9rem;
            color: #fce8f6;
            cursor: pointer;
            transition: background-color 0.2s ease;
            border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        }

        .dropdown-item:last-child {
            border-bottom: none;
        }

        .dropdown-item:hover {
            background-color: rgba(50, 6, 37, 0.8);
            border-color: rgba(255, 255, 255, 0.4);
            box-shadow: 0 2px 5px rgba(0, 0, 0, 0.3);
        }

        .dropdown-item.active {
            background-color: var(--bg-color);
            border-color: #fce8f6;
            box-shadow: 0 0 10px rgba(255, 255, 255, 0.2);
            color: #fff;
        }

        /* Fix for header-tabs to prevent dropdown from affecting it */
        .header-tabs {
            display: flex;
            gap: 5px;
            margin: 0 10px;
            flex-grow: 1;
            justify-content: center;
            flex-wrap: wrap;
            overflow-x: visible;
            scrollbar-width: thin;
            max-width: 70%;
            position: relative;
        }";
            
        css
            .replace("<BG_COLOR>", &bg_color)
            .replace("<BG_IMAGE>", &bg_image)
            .replace("<SECONDARY_FONT>", HEADER_FONT)
            .replace("<PRIMARY_FONT>", REGULAR_FONT)
            + "/* Font fixes applied */"
    };

    let mut modal_context = use_context_provider(ModalContext::default);
    if let Some(e) = err() {
        modal_context.open("Error", rsx! {
            p {
                "The installer encountered an error if the problem does not resolve itself please open a thread in #📂modpack-issues on the discord."
            }
            textarea { class: "error-area", readonly: true, "{e}" }
        }, false, Some(move |_| err.set(None)));
    }

    // Determine which logo to use
    let logo_url = Some("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/icon.png".to_string());
    
    // Fix: Return the JSX from the app function
    let current_page = page();
    debug!("RENDER DECISION: current_page={}, HOME_PAGE={}, is_home={}",
           current_page, HOME_PAGE, current_page == HOME_PAGE);
    
    rsx! {
        div {
            style { {css_content} }
            Modal {}

            BackgroundParticles {}
            
            {if !config.read().first_launch.unwrap_or(true) && launcher.is_some() && !settings() {
                rsx! {
                    AppHeader {
                        page,
                        pages,
                        settings,
                        logo_url
                    }
                }
            } else {
                None
            }}

            div { class: "main-container",
                {if settings() {
                    rsx! {
                        Settings {
                            config,
                            settings,
                            config_path: props.config_path.clone(),
                            error: err,
                            b64_id: URL_SAFE_NO_PAD.encode(props.modpack_source)
                        }
                    }
                } else if config.read().first_launch.unwrap_or(true) || launcher.is_none() {
                    rsx! {
                        Launcher {
                            config,
                            config_path: props.config_path.clone(),
                            error: err,
                            b64_id: URL_SAFE_NO_PAD.encode(props.modpack_source)
                        }
                    }
                } else if packs.read().is_none() {
                    rsx! {
                        div { class: "loading-container",
                            div { class: "loading-spinner" }
                            div { class: "loading-text", "Loading modpack information..." }
                        }
                    }
                } else {
                    // DIAGNOSTIC CONTENT RENDERING SECTION
                    if current_page == HOME_PAGE {
                        debug!("RENDERING: HomePage");
                        rsx! {
                            NewHomePage {
                installations: installations,
                error_signal: err
                            }
                        }
                    } else {
                        debug!("RENDERING: Content for page {}", current_page);
                        
                        // Get tab info without temporary references
                        let pages_map = pages();
                        
                        if let Some(tab_info) = pages_map.get(&current_page) {
                            debug!("FOUND tab group {} with {} modpacks", 
                                current_page, tab_info.modpacks.len());
                            
                            // CRITICAL FIX: Get all modpacks before rendering
                            let modpacks = tab_info.modpacks.clone();
                            debug!("Cloned {} modpacks for rendering", modpacks.len());
                            
                            // Log each modpack outside the RSX
                            for profile in &modpacks {
                                debug!("Preparing to render modpack: {}", profile.manifest.subtitle);
                            }
                            
                            // Create a separate credits signal for this rendering path
                            let mut credits_visible = use_signal(|| false);
                            let mut selected_profile = use_signal(|| modpacks.first().cloned());
                            let mut error_msg = use_signal(|| Option::<String>::None);
                            
                            // Directly return the RSX without unnecessary nesting
                            rsx! {
                                // First, conditionally render either the credits view or the normal content
                                if *credits_visible.read() {
                                    // Render the Credits component with the selected profile
                                    if let Some(profile) = selected_profile.read().clone() {
                                        Credits {
                                            manifest: profile.manifest.clone(),
                                            enabled: profile.enabled_features.clone(),
                                            credits: credits_visible
                                        }
                                    }
                                } else {
                                    // Error notification if any
                                    if let Some(error) = error_msg() {
                                        div { class: "error-notification",
                                            div { class: "error-message", "{error}" }
                                            button { 
                                                class: "error-close",
                                                onclick: move |_| error_msg.set(None),
                                                "×"
                                            }
                                        }
                                    }
                                    
                                    // Render the normal modpack content
                                    div { 
                                        class: "version-page-container",
                                        style: "display: block; width: 100%;",
                                        
                                        for (index, profile) in modpacks.iter().enumerate() {
                                            {
                                                let profile_clone = profile.clone();
                                                let is_installed = profile.installed;
                                                let uuid = profile.manifest.uuid.clone();
                                                let error_signal = error_msg.clone();
                                                
                                                rsx! {
                                                    div { 
                                                        class: "version-container",
                                                        
                                                        // Header section
                                                        div { class: "content-header",
                                                            h1 { "{profile.manifest.subtitle}" }
                                                        }
                                                        
                                                        // Description section
                                                        div { class: "content-description",
                                                            dangerous_inner_html: "{profile.manifest.description}"
                                                        }

                                                        // Credits link - moved outside the description HTML
                                                        div { class: "credits-link-container", style: "text-align: center; margin: 15px 0;",
                                                            a {
                                                                class: "credits-button",
                                                                onclick: move |evt| {
                                                                    // Set the selected profile and show credits
                                                                    selected_profile.set(Some(profile_clone.clone()));
                                                                    credits_visible.set(true);
                                                                    evt.stop_propagation();
                                                                },
                                                                "VIEW CREDITS"
                                                            }
                                                        }
                                                        
                                                        // Features heading
                                                        h2 { class: "features-heading", "OPTIONAL FEATURES" }
                                                        
                                                        // MODIFIED SECTION: Expandable Features
                                                        div { class: "features-section",
                                                            {
                                                                // Filter features inside the RSX block
                                                                let visible_features: Vec<_> = profile.manifest.features.iter()
                                                                    .filter(|f| !f.hidden)
                                                                    .collect();
                                                                
                                                                // Calculate whether to show expand button
                                                                let first_row_count = 3;
                                                                let show_expand_button = visible_features.len() > first_row_count;
                                                                
                                                                // Using a unique signal for each profile's expanded state
                                                                let expanded_signal_id = format!("expanded-{}-{}", current_page, index);
                                                                let mut expanded_features = use_signal(|| false);
                                                                
                                                                rsx! {
                                                                    div { class: "feature-cards-container",
                                                                        // Feature cards rendering (unchanged) 
                                                                        // ...
                                                                    }
                                                                    
                                                                    // Only show expand button if needed
                                                                    if show_expand_button {
                                                                        div { class: "features-expand-container",
                                                                            button {
                                                                                class: "features-expand-button",
                                                                                onclick: move |_| {
                                                                                    let current_state = *expanded_features.read();
                                                                                    expanded_features.set(!current_state);
                                                                                    debug!("Toggled expanded features: {} for profile {}", !current_state, expanded_signal_id);
                                                                                },
                                                                                if *expanded_features.read() {
                                                                                    "Collapse Features"
                                                                                } else {
                                                                                    {format!("Show {} More Features", visible_features.len() - first_row_count)}
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        
                                                        // Install button and Play button in sequence
                                                        div { 
                                                            class: "buttons-container",
                                                            style: "display: flex; flex-direction: column; align-items: center; margin-top: 20px;",
                                                            
                                                            // Install/Update/Modify button
                                                            div { class: "install-button-container",
                                                                div { class: "button-scale-wrapper",
                                                                    button { 
                                                                        class: "main-install-button",
                                                                        // You can add proper install logic here if needed
                                                                        if profile.installed && profile.update_available {
                                                                            "Update"
                                                                        } else if profile.installed {
                                                                            "Modify"
                                                                        } else {
                                                                            "Install"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            
                                                            // Play button (only if installed)
                                                            if is_installed {
                                                                {
                                                                    let uuid_str = uuid.clone(); // Clone outside
                                                                    let err_signal = error_signal.clone();
                                                                    
                                                                    rsx! {
                                                                        PlayButton {
                                                                            uuid: uuid_str.clone(), // Clone again for component
                                                                            disabled: false,
                                                                            auth_status: None,
                                                                            onclick: move |_| {
                                                                                let uuid_for_handler = uuid_str.clone(); // Clone inside closure
                                                                                handle_play_click(uuid_for_handler, &err_signal);
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
                                }
                            }
                        } else {
                            debug!("NO TAB INFO found for page {}", current_page);
                            rsx! { div { "No modpack information found for this tab." } }
                        }
                    }
                }
            }
        }
    }
}
}
