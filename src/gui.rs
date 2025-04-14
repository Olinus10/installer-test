use dioxus::prelude::*;
use std::fs;
use std::panic;
use std::backtrace::Backtrace;
use std::env;
use platform_info::{PlatformInfo, PlatformInfoAPI, UNameAPI};
use simplelog::{CombinedLogger, TermLogger, WriteLogger, LevelFilter, TerminalMode, ColorChoice, Config as LogConfig};
use std::fs::File;
use dioxus::desktop::{Config as DioxusConfig, WindowBuilder, LogicalSize};
use dioxus::desktop::tao::window::Icon;
use std::collections::BTreeMap;
use std::path::PathBuf;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use modal::ModalContext;
use modal::Modal; 
use std::sync::mpsc;
use log::{debug, error, info};
use isahc::ReadResponseExt;

use crate::{GithubBranch, build_http_client, GH_API, REPO, Config};
use crate::{get_app_data, get_installed_packs, get_launcher, uninstall, InstallerProfile, Launcher, PackName};
use crate::Installation;
use crate::installation;
use crate::universal;
use crate::CachedHttpClient;
use crate::changelog::{fetch_changelog, Changelog as ChangelogData};
use crate::preset;
use crate::launch_modpack;
use crate::universal::ModComponent;
use crate::universal::ManifestError;
use crate::universal::ManifestErrorType;
use crate::launcher::FeaturesTab;
use crate::launcher::PerformanceTab;
use crate::launcher::SettingsTab;

mod modal;

// Font constants
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
fn PlayButton(
    uuid: String,
    disabled: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {    
    rsx! {
        div { class: "play-button-container",
            button {
                class: "main-play-button",
                disabled: disabled,
                onclick: move |evt| onclick.call(evt),
                
                "PLAY"
            }
        }
    }
}

// Play button handler
pub fn handle_play_click(uuid: String, error_signal: &Signal<Option<String>>) {
    debug!("Play button clicked for modpack: {}", uuid);
    
    // Create a channel to communicate back to the main thread
    let (error_tx, error_rx) = mpsc::channel::<String>();
    
    // Clone error_signal before moving to thread
    let mut error_signal_clone = error_signal.clone();
    
    // Launch the game directly without auth
    let uuid_clone = uuid.clone();
    std::thread::spawn(move || {
        match crate::launch_modpack(&uuid_clone) {
            Ok(_) => {
                debug!("Successfully launched modpack: {}", uuid_clone);
            },
            Err(e) => {
                error!("Failed to launch modpack: {}", e);
                let _ = error_tx.send(format!("Failed to launch modpack: {}", e));
            }
        }
    });
    
    // Create a task to check for errors from the background thread
    spawn(async move {
        if let Ok(error_message) = error_rx.recv() {
            error_signal_clone.set(Some(error_message));
        }
    });
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
fn ErrorNotification(
    message: String,
    on_close: EventHandler<MouseEvent>,
) -> Element {
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

#[component]
fn ManifestErrorDisplay(
    error: String,
    error_type: String,
    file_name: String,
    onclose: EventHandler<MouseEvent>,
    onreport: EventHandler<MouseEvent>,
) -> Element {
    let formatted_error = format_error_message(&error);
    
    rsx! {
        div { class: "manifest-error-overlay",
            div { class: "manifest-error-container",
                div { class: "manifest-error-header",
                    h2 { "{error_type} Error" }
                    button { 
                        class: "close-button",
                        onclick: move |evt| onclose.call(evt),
                        "×"
                    }
                }
                
                div { class: "manifest-error-content",
                    p { class: "error-intro",
                        "There was a problem loading the {file_name} file. This could be due to:"
                    }
                    
                    ul { class: "error-reasons",
                        li { "A network connection issue" }
                        li { "A formatting problem in the file" }
                        li { "Missing or invalid data in the file" }
                    }
                    
                    div { class: "error-details-container",
                        h3 { "Error Details" }
                        div { class: "error-details",
                            pre { "{formatted_error}" }
                        }
                    }
                    
                    p { class: "error-help",
                        "Please copy these error details and report this issue so we can fix it."
                    }
                }
                
                div { class: "manifest-error-footer",
                    button { 
                        class: "report-button",
                        onclick: move |evt| onreport.call(evt),
                        "Report Issue"
                    }
                    
                    button { 
                        class: "copy-button",
                        onclick: move |_| {
                            // This would need to be implemented in your system
                            // Typically using web_sys::clipboard in WASM
                            debug!("Copying error to clipboard");
                        },
                        "Copy Error Details"
                    }
                }
            }
        }
    }
}

// 2. Add a utility function to format error messages in a user-friendly way
fn format_error_message(error: &str) -> String {
    // Extract the most useful information from the error message
    
    if error.contains("failed to deserialize") || error.contains("missing field") {
        // Handle deserialization errors
        if let Some(field_name) = extract_field_name(error) {
            return format!("Problem with field: '{}'\n\nFull error: {}", field_name, error);
        }
    } else if error.contains("expected value") && error.contains("true") {
        return format!("Boolean value error: JSON requires lowercase 'true' and 'false'.\n\nFull error: {}", error);
    } else if error.contains("HTTP 404") {
        return format!("File not found (404). The file may have moved or been renamed.\n\nFull error: {}", error);
    } else if error.to_lowercase().contains("unexpected character") {
        return format!("JSON syntax error. There may be a missing comma, quote, or bracket.\n\nFull error: {}", error);
    }
    
    // Default case - return the original error
    error.to_string()
}

// Extract field name from deserialization errors
fn extract_field_name(error: &str) -> Option<String> {
    // Try to extract field name from common error patterns
    let patterns = [
        "missing field `", 
        "unknown field `",
        "invalid type for field `"
    ];
    
    for pattern in patterns {
        if let Some(start) = error.find(pattern) {
            let start_pos = start + pattern.len();
            if let Some(end) = error[start_pos..].find('`') {
                return Some(error[start_pos..(start_pos + end)].to_string());
            }
        }
    }
    
    None
}

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        match std::process::Command::new("cmd")
            .args(&["/c", "start", "", url])
            .spawn() {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to open URL: {}", e)),
            }
    }
    
    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("open")
            .arg(url)
            .spawn() {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to open URL: {}", e)),
            }
    }
    
    #[cfg(target_os = "linux")]
    {
        match std::process::Command::new("xdg-open")
            .arg(url)
            .spawn() {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to open URL: {}", e)),
            }
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Opening URLs is not supported on this platform".to_string())
    }
}

// Updated main function to initialize the app correctly
fn main() {
    // Initialize logger
    fs::create_dir_all(get_app_data().join(".WC_OVHL/")).expect("Failed to create config dir!");
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Debug,
            simplelog::ConfigBuilder::new().add_filter_ignore_str("isahc::handler").build(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            LogConfig::default(),
            File::create(get_app_data().join(".WC_OVHL/installer.log")).unwrap(),
        ),
    ])
    .unwrap();

    // Set panic hook for error reporting
    panic::set_hook(Box::new(|info| {
        let payload = if let Some(string) = info.payload().downcast_ref::<String>() {
            string.to_string()
        } else if let Some(str) = info.payload().downcast_ref::<&'static str>() {
            str.to_string()
        } else {
            format!("{:?}", info.payload())
        };
        let backtrace = Backtrace::force_capture();
        error!("The installer panicked! This is a bug.\n{info:#?}\nPayload: {payload}\nBacktrace: {backtrace}");
    }));

    // Log initialization info
    info!("Installer version: {}", env!("CARGO_PKG_VERSION"));
    let platform_info = PlatformInfo::new().expect("Unable to determine platform info");
    debug!("System information:\n\tSysname: {}\n\tRelease: {}\n\tVersion: {}\n\tArchitecture: {}\n\tOsname: {}",
        platform_info.sysname().to_string_lossy(), 
        platform_info.release().to_string_lossy(), 
        platform_info.version().to_string_lossy(), 
        platform_info.machine().to_string_lossy(), 
        platform_info.osname().to_string_lossy()
    );

    // Workaround for NVIDIA driver issues on Linux
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/dev/dri").exists() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            warn!("Disabled hardware acceleration as a workaround for NVIDIA driver issues")
        }
    }

    // Load icon
    let icon = image::load_from_memory(include_bytes!("assets/icon.png")).unwrap();
    
    // Load branches
    let branches: Vec<GithubBranch> = serde_json::from_str(
        build_http_client()
            .get(GH_API.to_owned() + REPO + "branches")
            .expect("Failed to retrieve branches!")
            .text()
            .unwrap()
            .as_str(),
    )
    .expect("Failed to parse branches!");

    // Load configuration
    let config_path = get_app_data().join(".WC_OVHL/config.json");
    let config: Config;

    // Load or create config
    if config_path.exists() {
        config = serde_json::from_slice(&fs::read(&config_path).expect("Failed to read config!"))
            .expect("Failed to load config!");
    } else {
        config = Config {
            launcher: String::from("vanilla"),
            first_launch: Some(true),
        };
        fs::write(&config_path, serde_json::to_vec(&config).unwrap())
            .expect("Failed to write config!");
    }
    
    info!("Running installer with config: {config:#?}");
    
    // Load all installations (or empty vector if error)
    let installations = installation::load_all_installations().unwrap_or_default();
    
    // Launch the UI
    LaunchBuilder::desktop().with_cfg(
        DioxusConfig::new().with_window(
            WindowBuilder::new()
                .with_resizable(true)
                .with_title("Majestic Overhaul Launcher")
                .with_inner_size(LogicalSize::new(1280, 720))
                .with_min_inner_size(LogicalSize::new(960, 540))
        ).with_icon(
            Icon::from_rgba(icon.to_rgba8().to_vec(), icon.width(), icon.height()).unwrap(),
        ).with_data_directory(
            env::temp_dir().join(".WC_OVHL")
        ).with_menu(None)
    ).with_context(AppProps {
        branches,
        modpack_source: String::from(REPO),
        config,
        config_path,
        installations,
    }).launch(app);
}

#[component]
fn ChangelogSection(changelog: Option<ChangelogData>) -> Element {
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
                            view_box: "0 0 24 24",
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
fn HomePage(
    installations: Signal<Vec<Installation>>,
    error_signal: Signal<Option<String>>,
    changelog: Signal<Option<ChangelogData>>,
    current_installation_id: Signal<Option<String>>, // Add this parameter
) -> Element {
    // State for the installation creation dialog
    let mut show_creation_dialog = use_signal(|| false);
    
    // Check if this is the first time (no installations)
    let has_installations = !installations().is_empty();
    let latest_installation = installations().first().cloned();
    
    rsx! {
        div { class: "home-container",
            // Error notification if any
            if let Some(error) = error_signal() {
                ErrorNotification {
                    message: error,
                    on_close: move |_| {
                        error_signal.set(None);
                    }
                }
            }
            
            if has_installations {
                // Regular home page with installations
                // Welcome header
                div { class: "welcome-header",
                    h1 { "Welcome to Wynncraft Overhaul" }
                }
                
                // Statistics display
                StatisticsDisplay {}
                
                // Section divider for installations
                div { class: "section-divider with-title", 
                    span { class: "divider-title", "YOUR INSTALLATIONS" }
                }
                
                // Big play button for the most recent installation
                if let Some(installation) = latest_installation.clone() {
                    div { class: "main-play-container",
                        // Play button
                        {
                            let play_id = installation.id.clone();
                            rsx! {
                                PlayButton {
                                    uuid: play_id.clone(),
                                    disabled: false,
                                    onclick: move |_| {
                                        let play_id_inner = play_id.clone();
                                        handle_play_click(play_id_inner, &error_signal);
                                    }
                                }
                            }
                        }
                        
                        // Quick update button if available
                        if installation.update_available {
                            {
                                let update_id = installation.id.clone();
                                rsx! {
                                    button {
                                        class: "quick-update-button",
                                        onclick: move |_| {
                                            // Quick update functionality
                                            debug!("Quick update clicked for: {}", update_id);
                                        },
                                        "Update Available"
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Grid of installation cards
                div { class: "installations-grid",
                    // Existing installation cards
                    for installation in installations() {
                        {
                            let installation_id = installation.id.clone();
                            rsx! {
                                InstallationCard { 
                                    installation: installation.clone(),
                                    onclick: move |_| {
                                        debug!("Clicked installation: {}", installation_id);
                                        current_installation_id.set(Some(installation_id.clone()));
                                    }
                                }
                            }
                        }
                    }
                    
                    // Create new installation card
                    div { 
                        class: "installation-card new-installation",
                        onclick: move |_| show_creation_dialog.set(true),
                        
                        div { class: "installation-card-content", 
                            div { class: "installation-card-icon", "+" }
                            h3 { "Create New Installation" }
                            p { "Set up a new Wynncraft experience" }
                        }
                    }
                }
            } else {
                // First-time user experience
                div { class: "welcome-container first-time",
                    h1 { "Welcome to Wynncraft Overhaul" }
                    p { "Enhance your Wynncraft experience with optimized performance and improved visuals." }
                    
                    // Statistics for first-time users too
                    StatisticsDisplay {}
                    
                    button {
                        class: "main-install-button",
                        onclick: move |_| {
                            show_creation_dialog.set(true);
                        },
                        "Get Started"
                    }
                }
            }
            
            // Recent changes section
            ChangelogSection { changelog: changelog() }
            
            // Footer with Discord button and other info
            Footer {}
            
            // Installation creation dialog
            if *show_creation_dialog.read() {
    SimplifiedInstallationWizard {
        onclose: move |_| {
            show_creation_dialog.set(false);
        },
        oncreate: move |new_installation: Installation| {  // Added type annotation here
            // Add the new installation to the list
            installations.with_mut(|list| {
                list.insert(0, new_installation.clone());
            });
            
            // Close the dialog
            show_creation_dialog.set(false);
            
            // Set the current installation to navigate to the installation page
            current_installation_id.set(Some(new_installation.id));
                    }
                }
            }
        }
    }
}

// Special value for home page
const HOME_PAGE: usize = usize::MAX;

#[component]
fn InstallationCard(
    installation: Installation,
    onclick: EventHandler<String>,
) -> Element {
    // Format last played date
    let last_played = installation.last_launch.map(|dt| {
        dt.format("%B %d, %Y").to_string()
    });
    
    // Clone the ID for event handlers
    let installation_id = installation.id.clone();
    let play_id = installation.id.clone();
    let mut error_signal = use_signal(|| Option::<String>::None);
    
    rsx! {
        div { 
            class: "installation-card",
            "data-id": "{installation.id}",
            
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
                
                div { class: "detail-item memory-detail",
                    span { class: "detail-label", "Memory:" }
                    span { class: "detail-value", "{installation.memory_allocation} MB" }
                }
            }
            
            div { class: "installation-card-actions",
                button { 
                    class: "play-button",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        handle_play_click(play_id.clone(), &error_signal);
                    },
                    "Play"
                }
                
                button { 
                    class: "manage-button",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        onclick.call(installation_id.clone());
                    },
                    "Manage"
                }
            }
            
            // Display error if any occurs during play
            if let Some(error) = error_signal() {
                ErrorNotification {
                    message: error,
                    on_close: move |_| {
                        error_signal.set(None);
                    }
                }
            }
        }
    }
}

// Statistics display component
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

// Fixed: Combined and unified this struct to remove duplication
#[derive(PartialEq, Props, Clone)]
pub struct InstallationCreationProps {
    pub onclose: EventHandler<()>,
    pub oncreate: EventHandler<Installation>,
}

#[component]
pub fn SimplifiedInstallationWizard(props: InstallationCreationProps) -> Element {
    // State for installation name only
    let mut name = use_signal(|| "My Wynncraft Installation".to_string());
    let mut installation_error = use_signal(|| Option::<String>::None);
    let mut manifest_error = use_signal(|| Option::<ManifestError>::None);
    
    // Resource for universal manifest with better error handling
    let manifest_error_clone = manifest_error.clone();
    let universal_manifest = use_resource(move || {
        let mut manifest_error = manifest_error_clone.clone();
        async move {
            debug!("Loading universal manifest...");
            match crate::universal::load_universal_manifest(
                &crate::CachedHttpClient::new(), 
                Some("https://raw.githubusercontent.com/Olinus10/installer-test/master/universal.json")
            ).await {
                Ok(manifest) => {
                    debug!("Successfully loaded universal manifest: {}", manifest.name);
                    Some(manifest)
                },
                Err(e) => {
                    error!("Failed to load universal manifest: {}", e);
                    // Use spawn to update the signal from outside the closure
                    spawn(async move {
                        manifest_error.set(Some(e.clone()));
                    });
                    None
                }
            }
        }
    });
    
    // Function to create the installation
    let create_installation = move |_| {
        debug!("Creating installation with name: {}", name.read());
        
        // Get the universal manifest for Minecraft version and loader information
        if let Some(unwrapped_manifest) = universal_manifest.read().as_ref().and_then(|opt| opt.as_ref()) {
            let minecraft_version = unwrapped_manifest.minecraft_version.clone();
            let loader_type = unwrapped_manifest.loader.r#type.clone();
            let loader_version = unwrapped_manifest.loader.version.clone();
            
            // Create a basic installation
            let installation = Installation::new_custom(
                name.read().clone(),
                minecraft_version,
                loader_type,
                loader_version,
                "vanilla".to_string(),
                unwrapped_manifest.version.clone(),
            );
            
            // Register the installation
            if let Err(e) = crate::installation::register_installation(&installation) {
                error!("Failed to register installation: {}", e);
                installation_error.set(Some(format!("Failed to register installation: {}", e)));
                return;
            }
            
            // Save the installation
            if let Err(e) = installation.save() {
                error!("Failed to save installation: {}", e);
                installation_error.set(Some(format!("Failed to save installation: {}", e)));
                return;
            }
            
            debug!("Successfully created installation: {}", installation.id);
            
            // Call the oncreate handler to finalize
            props.oncreate.call(installation);
        } else {
            error!("Universal manifest not available");
            installation_error.set(Some("Failed to load modpack information. Please try again.".to_string()));
        }
    };
    
    rsx! {
        div { class: "wizard-overlay",
            div { class: "installation-wizard",
                // Header
                div { class: "wizard-header",
                    h2 { "Create New Installation" }
                    button { 
                        class: "close-button",
                        onclick: move |_| props.onclose.call(()),
                        "×"
                    }
                }
                
                // Error notification if any
                if let Some(error) = &*installation_error.read() {
                    div { class: "error-message",
                        "{error}"
                        button {
                            class: "error-close",
                            onclick: move |_| installation_error.set(None),
                            "×"
                        }
                    }
                }
                
                // Main content - simplified to just name
                div { class: "wizard-content",
                    // Name section
                    div { class: "wizard-section",
                        h3 { "Installation Name" }
                        div { class: "form-group",
                            label { r#for: "installation-name", "Name your installation:" }
                            input {
                                id: "installation-name",
                                r#type: "text",
                                value: "{name}",
                                oninput: move |evt| name.set(evt.value().clone()),
                                placeholder: "e.g. My Wynncraft Adventure"
                            }
                        }
                    }
                    
                    // Minecraft info section - just to show what they're creating
                    if let Some(unwrapped_manifest) = universal_manifest.read().as_ref().and_then(|opt| opt.as_ref()) {
                        div { class: "wizard-section minecraft-info",
                            div { class: "info-row",
                                div { class: "info-item",
                                    span { class: "info-label", "Minecraft:" }
                                    span { class: "info-value", "{unwrapped_manifest.minecraft_version}" }
                                }
                                
                                div { class: "info-item",
                                    span { class: "info-label", "Loader:" }
                                    span { class: "info-value", "{unwrapped_manifest.loader.r#type} {unwrapped_manifest.loader.version}" }
                                }
                            }
                        }
                    } else {
                        div { class: "loading-section",
                            div { class: "loading-spinner" }
                            div { class: "loading-text", "Loading modpack information..." }
                        }
                    }
                }
                
                // Footer with create button
                div { class: "wizard-footer",
                    button {
                        class: "cancel-button",
                        onclick: move |_| props.onclose.call(()),
                        "Cancel"
                    }
                    
                    button {
                        class: "create-button",
                        disabled: universal_manifest.read().is_none(),
                        onclick: create_installation,
                        "Create Installation"
                    }
                }
            }
            
            // Add manifest error display
            if let Some(error) = manifest_error() {
                ManifestErrorDisplay {
                    error: error.message.clone(),
                    error_type: format!("{}", error.error_type),
                    file_name: error.file_name.clone(),
                    onclose: move |_| manifest_error.set(None),
                    onreport: move |_| {
                        let _ = open_url("https://discord.com/channels/778965021656743966/1234506784626970684");
                    }
                }
            }
        }
    }
}

// Installation management page
#[component]
pub fn InstallationManagementPage(
    installation_id: String,
    onback: EventHandler<()>,
    installations: Signal<Vec<Installation>>,
) -> Element {
    // State for the current tab
    let mut active_tab = use_signal(|| "features");
    
    // Load the installation data
    let installation_result = use_memo(move || {
        crate::installation::load_installation(&installation_id)
    });

    // Installation status signals
    let mut is_installing = use_signal(|| false);
    let mut installation_error = use_signal(|| Option::<String>::None);
    
    // Handle installation not found
    if let Err(e) = &*installation_result.read() {
        return rsx! {
            div { class: "error-container",
                h2 { "Installation Not Found" }
                p { "The requested installation could not be found." }
                p { "Error: {e}" }
                
                button {
                    class: "back-button",
                    onclick: move |_| onback.call(()),
                    "Back to Home"
                }
            }
        };
    } 
    
    // Unwrap installation from result
    let installation = installation_result.read().as_ref().unwrap().clone();
    
    // Clone needed values to avoid partial moves
    let installation_id_for_delete = installation.id.clone();
    let installation_id_for_launch = installation.id.clone();
    let installation_for_update = installation.clone();
    
    // State for modification tracking
    let mut has_changes = use_signal(|| false);
    let mut enabled_features = use_signal(|| installation.enabled_features.clone());
    let mut memory_allocation = use_signal(|| installation.memory_allocation);
    let mut java_args = use_signal(|| installation.java_args.clone());
    let mut selected_preset = use_signal(|| Option::<String>::None);
    
    // State for tracking modifications in different areas 
    let mut features_modified = use_signal(|| false);
    let mut performance_modified = use_signal(|| false);
    
    // Filter text for feature search
    let mut filter_text = use_signal(|| String::new());

    let refresh_installation = move |updated_installation: Installation| {
    // Update the current installation data
    installations.with_mut(|list| {
    if let Some(index) = list.iter().position(|i| i.id == updated_installation.id) {
        list[index] = updated_installation.clone();
    }
});
    
    // Also update the list of installations
    installations.with_mut(|list| {
        // Find and replace the installation
        if let Some(index) = list.iter().position(|i| i.id == updated_installation.id) {
            list[index] = updated_installation;
        }
    });
};

let update_installation = move |updated: Installation| {
    // Update in the installations list
    installations.with_mut(|list| {
        if let Some(index) = list.iter().position(|i| i.id == updated.id) {
            list[index] = updated.clone();
        }
    });
    
    // Reload the current view
    spawn(async move {
        match installation::load_installation(&updated.id) {
            Ok(_refreshed) => {
                // The list update above already updates the installation
                // No need to do anything extra here
            },
            Err(e) => {
                debug!("Failed to reload installation: {}", e);
            }
        }
    });
};
    
    // Load universal manifest for features information
    let universal_manifest = use_resource(move || async {
        match crate::universal::load_universal_manifest(&crate::CachedHttpClient::new(), None).await {
            Ok(manifest) => {
                debug!("Successfully loaded universal manifest for features");
                Some(manifest)
            },
            Err(e) => {
                error!("Failed to load universal manifest: {}", e);
                None
            }
        }
    });
    
    // Load presets
    let presets = use_resource(move || async {
        match crate::preset::load_presets(&crate::CachedHttpClient::new(), None).await {
            Ok(presets) => {
                debug!("Successfully loaded {} presets", presets.len());
                presets
            },
            Err(e) => {
                error!("Failed to load presets: {}", e);
                Vec::new()
            }
        }
    });
    
    // Effect to detect changes
    use_effect({
        let enabled_features_for_effect = enabled_features.clone();
        let java_args_for_effect = java_args.clone();
        let original_features = installation.enabled_features.clone();
        let original_java_args = installation.java_args.clone();
        let memory_allocation_for_effect = memory_allocation.clone();
        let original_memory = installation.memory_allocation;
        let mut features_modified_copy = features_modified.clone();
        let mut performance_modified_copy = performance_modified.clone();
        
        move || {
            let features_changed = enabled_features_for_effect.read().clone() != original_features;
            let memory_changed = *memory_allocation_for_effect.read() != original_memory;
            let args_changed = *java_args_for_effect.read() != original_java_args;
            
            // Update specific modification flags
            features_modified_copy.set(features_changed);
            performance_modified_copy.set(memory_changed || args_changed);
            
            // Update overall change flag
            has_changes.set(features_changed || memory_changed || args_changed);
        }
    });
    
    // Handle install/update
    let handle_update = move |_| {
        is_installing.set(true);
        let mut installation_clone = installation_for_update.clone();
        
        // Update settings
        installation_clone.enabled_features = enabled_features.read().clone();
        installation_clone.memory_allocation = *memory_allocation.read();
        installation_clone.java_args = java_args.read().clone();
        installation_clone.modified = true;
        
        let http_client = crate::CachedHttpClient::new();
        let mut installation_error_clone = installation_error.clone();
        
        spawn(async move {
            match installation_clone.install_or_update(&http_client).await {
                Ok(_) => {
                    debug!("Successfully updated installation: {}", installation_clone.id);
                    // Save the updated installation
                    if let Err(e) = installation_clone.save() {
                        error!("Failed to save changes: {}", e);
                        installation_error_clone.set(Some(format!("Failed to save changes: {}", e)));
                    } else {
                        has_changes.set(false);
                        features_modified.set(false);
                        performance_modified.set(false);
                    }
                },
                Err(e) => {
                    error!("Failed to update installation: {}", e);
                    installation_error_clone.set(Some(e));
                }
            }
            is_installing.set(false);
        });
    };
    
    // Button label based on state
    let action_button_label = if !installation.installed {
        "Install"
    } else if installation.update_available {
        "Update"
    } else if *has_changes.read() {
        "Apply Changes"
    } else {
        "Already Up To Date"
    };
    
    // Button disable logic
    let action_button_disabled = *is_installing.read() || 
                                (!installation.update_available && 
                                 installation.installed && 
                                 !*has_changes.read());
    
    // Handle launch
    let handle_launch = move |_| {
        let mut installation_error_clone = installation_error.clone();
        let installation_id = installation_id_for_launch.clone();
        
        // Create a channel to communicate back to the main thread
        let (error_tx, error_rx) = std::sync::mpsc::channel::<String>();
        
        // Launch the game
        std::thread::spawn(move || {
            match crate::launch_modpack(&installation_id) {
                Ok(_) => {
                    debug!("Successfully launched modpack: {}", installation_id);
                },
                Err(e) => {
                    error!("Failed to launch modpack: {}", e);
                    let _ = error_tx.send(format!("Failed to launch modpack: {}", e));
                }
            }
        });
        
        // Create a task to check for errors from the background thread
        spawn(async move {
            if let Ok(error_message) = error_rx.recv() {
                installation_error_clone.set(Some(error_message));
            }
        });
    };
    
    rsx! {
        div { class: "installation-management-container",
            // Back navigation
            div { class: "navigation-row",
                button { 
                    class: "back-button",
                    onclick: move |_| onback.call(()),
                    "← Back to Installations"
                }
            }
            
            // Header with installation name
            div { class: "installation-header",
                h1 { "{installation.name}" }
                div { class: "installation-meta",
                    span { class: "minecraft-version", "Minecraft {installation.minecraft_version}" }
                    span { class: "loader-version", "{installation.loader_type} {installation.loader_version}" }
                    
                    if installation.update_available {
                        span { class: "update-badge", "Update Available" }
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
            
            // Main tabs and content area
            div { class: "installation-content-container",
                // Tab navigation
                div { class: "installation-tabs",
                    button { 
                        class: if *active_tab.read() == "features" { "tab-button active" } else { "tab-button" },
                        onclick: move |_| active_tab.set("features"),
                        "Features & Presets"
                        
                        // Show indicator if features modified
                        if *features_modified.read() {
                            span { class: "modified-indicator" }
                        }
                    }
                    button { 
                        class: if *active_tab.read() == "performance" { "tab-button active" } else { "tab-button" },
                        onclick: move |_| active_tab.set("performance"),
                        "Performance"
                        
                        // Show indicator if performance settings modified
                        if *performance_modified.read() {
                            span { class: "modified-indicator" }
                        }
                    }
                    button { 
                        class: if *active_tab.read() == "settings" { "tab-button active" } else { "tab-button" },
                        onclick: move |_| active_tab.set("settings"),
                        "Settings"
                    }
                }
                
                // Content area
                div { class: "installation-content",
                    match *active_tab.read() {
                        "features" => {
                            rsx! {
                                FeaturesTab {
                                    universal_manifest: universal_manifest.read().clone().flatten(),
                                    presets: presets.read().clone().unwrap_or_default(),
                                    enabled_features: enabled_features,
                                    selected_preset: selected_preset,
                                    filter_text: filter_text,
                                }
                            }
                        },
                        "performance" => {
                            rsx! {
                                PerformanceTab {
                                    memory_allocation: memory_allocation,
                                    java_args: java_args,
                                }
                            }
                        },
                        "settings" => {
                            rsx! {
                                SettingsTab {
    installation: installation.clone(),
    installation_id: installation_id_for_delete.clone(),
    ondelete: move |_| {
        // Handle delete functionality
        debug!("Delete clicked for: {}", installation_id_for_delete);
        onback.call(());
    },
    onupdate: move |updated_installation: Installation| {
        // Update the installation data
        installations.with_mut(|list| {
            if let Some(index) = list.iter().position(|i| i.id == updated_installation.id) {
                list[index] = updated_installation.clone();
            }
        });
    }
}
                    });
                },
                Err(e) => {
                    error!("Failed to reload installation: {}", e);
                }
            }
        });
    }
}
                            }
                        },
                        _ => rsx! { div { "Unknown tab selected" } }
                    }
                }
            }
            
            // Bottom action bar with install/update/modify button
            div { class: "installation-actions",
                // Launch button
                button {
                    class: "launch-button",
                    disabled: !installation.installed || *is_installing.read(),
                    onclick: handle_launch,
                    "Launch Game"
                }
                
                // Install/Update/Modify button
                button {
                    class: if installation.update_available {
                        "action-button update-button"
                    } else if *has_changes.read() {
                        "action-button modify-button"
                    } else {
                        "action-button"
                    },
                    disabled: action_button_disabled,
                    onclick: handle_update,
                    
                    if *is_installing.read() {
                        "Installing..."
                    } else {
                        {action_button_label}
                    }
                }
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
                                            // FIXED: Proper handling of href and author name with comma
                                            {
                                                let is_last = author == r#mod.authors.last().unwrap();
                                                rsx! {
                                                    a { 
                                                        href: author.link.clone(), 
                                                        class: "credit-author",
                                                        target: "_blank",
                                                        rel: "noopener noreferrer",
                                                        {format!("{}{}", author.name, if !is_last { ", " } else { "" })}
                                                    }
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
                                            // FIXED: Proper handling of href and author name with comma
                                            {
                                                let is_last = author == shaderpack.authors.last().unwrap();
                                                rsx! {
                                                    a { 
                                                        href: author.link.clone(), 
                                                        class: "credit-author",
                                                        target: "_blank",
                                                        rel: "noopener noreferrer",
                                                        {format!("{}{}", author.name, if !is_last { ", " } else { "" })}
                                                    }
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
                                            // FIXED: Proper handling of href and author name with comma
                                            {
                                                let is_last = author == resourcepack.authors.last().unwrap();
                                                rsx! {
                                                    a { 
                                                        href: author.link.clone(), 
                                                        class: "credit-author",
                                                        target: "_blank",
                                                        rel: "noopener noreferrer",
                                                        {format!("{}{}", author.name, if !is_last { ", " } else { "" })}
                                                    }
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
                                            // FIXED: Proper handling of href and author name with comma
                                            {
                                                let is_last = author == include.authors.as_ref().unwrap().last().unwrap();
                                                rsx! {
                                                    a { 
                                                        href: author.link.clone(), 
                                                        class: "credit-author",
                                                        target: "_blank",
                                                        rel: "noopener noreferrer",
                                                        {format!("{}{}", author.name, if !is_last { ", " } else { "" })}
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

    // Debug counter to force refreshes
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
    
    // Pre-build feature cards to avoid nested RSX macros
    let feature_cards_content = {
        // Filter features
        let features_list = features.read();
        let visible_features: Vec<_> = features_list.iter()
            .filter(|f| !f.hidden)
            .collect();
        
        // Calculate whether to show expand button
        let show_expand_button = visible_features.len() > first_row_count;
        
        let first_row_cards = visible_features.iter().take(first_row_count).map(|feat| {
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
        }).collect::<Vec<_>>();
        
        // Additional features (shown only when expanded)
        let additional_cards = if *expanded_features.read() {
            visible_features.iter().skip(first_row_count).map(|feat| {
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
            }).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        
        // Expand button
        let expand_button = if show_expand_button {
            vec![rsx! {
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
            }]
        } else {
            Vec::new()
        };
        
        // Combine all components
        let mut all_components = Vec::new();
        all_components.extend(first_row_cards);
        all_components.extend(additional_cards);
        all_components.extend(expand_button);
        
        all_components
    };
    
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
                        
                        // Feature cards container - using pre-built content instead of nested RSX
                        div { class: "feature-cards-container",
                            {feature_cards_content.into_iter()}
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
    installations: Signal<Vec<Installation>>,
    current_installation_id: Signal<Option<String>>,
    on_select_installation: EventHandler<String>,
    on_go_home: EventHandler<()>,
    on_open_settings: EventHandler<()>,
) -> Element {
    // Number of installation tabs to show directly
    let MAX_INSTALLATION_TABS = 3;
    
    // Prepare installation tabs
    let all_installations = installations();
    let direct_installations = all_installations.iter().take(MAX_INSTALLATION_TABS).cloned().collect::<Vec<_>>();
    let dropdown_installations = all_installations.iter().skip(MAX_INSTALLATION_TABS).cloned().collect::<Vec<_>>();
    
    // Current ID for active state
    let current_id = current_installation_id();
    
    // Pre-build direct tabs
    let direct_tabs_content = {
        let mut tabs = Vec::new();
        for installation in &direct_installations {
            let id = installation.id.clone();
            let name = installation.name.clone();
            let is_active = current_id.as_ref().map_or(false, |current_id| current_id == &id);
            let on_select = on_select_installation.clone();
            
            tabs.push(
                rsx! {
                    button {
                        class: {
                            if is_active { 
                                "header-tab-button active" 
                            } else { 
                                "header-tab-button" 
                            }
                        },
                        onclick: move |_| on_select.call(id.clone()),
                        "{name}"
                    }
                }
            );
        }
        tabs.into_iter()
    };
    
    // Pre-build dropdown menu
    let dropdown_menu = if !dropdown_installations.is_empty() {
        let dropdown_items = dropdown_installations.iter().map(|installation| {
            let id = installation.id.clone();
            let name = installation.name.clone();
            let is_active = current_id.as_ref().map_or(false, |current_id| current_id == &id);
            let on_select = on_select_installation.clone();
            
            rsx! {
                button {
                    class: {
                        if is_active { 
                            "dropdown-item active" 
                        } else { 
                            "dropdown-item" 
                        }
                    },
                    onclick: move |_| on_select.call(id.clone()),
                    "{name}"
                }
            }
        }).collect::<Vec<_>>();
        
        rsx! {
            div { class: "dropdown",
                button { class: "header-tab-button", "More Installations ▼" }
                div { class: "dropdown-content",
                    {dropdown_items.into_iter()}
                }
            }
        }
    } else {
        rsx! { Fragment {} }
    };
    
    // Main render
    rsx! {
        header { class: "app-header",
            // Logo and title
            div { 
                class: "app-header-left", 
                onclick: move |_| on_go_home.call(()),
                
                img { 
                    class: "app-logo", 
                    src: "/assets/logo.png", 
                    alt: "Wynncraft Overhaul Logo"
                }
                h1 { class: "app-title", "MAJESTIC OVERHAUL" }
            }
            
            // Tabs
            div { class: "header-tabs",
                // Home tab
                button { 
                    class: {
                        if current_id.is_none() { 
                            "header-tab-button active" 
                        } else { 
                            "header-tab-button" 
                        }
                    },
                    onclick: move |_| on_go_home.call(()),
                    "Home"
                }
                
                // Direct installation tabs
                {direct_tabs_content}
                
                // Dropdown menu
                {dropdown_menu}
                
                // Create new installation tab
                button { 
                    class: "header-tab-button new-installation-tab",
                    onclick: move |_| on_select_installation.call("new".to_string()),
                    "+"
                }
            }
            
            // Settings button
            button { 
                class: "settings-button",
                onclick: move |_| on_open_settings.call(()),
                "Settings"
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppProps {
    pub branches: Vec<super::GithubBranch>,
    pub modpack_source: String,
    pub config: super::Config,
    pub config_path: PathBuf,
    pub installations: Vec<Installation>,
}

// Fixed app function
pub fn app() -> Element {
    let props = use_context::<AppProps>();
    let css = include_str!("assets/style.css");
    
    // State management
    let config = use_signal(|| props.config);
    let mut settings = use_signal(|| false);
    let mut error_signal = use_signal(|| Option::<String>::None);
    let mut manifest_error = use_signal(|| Option::<ManifestError>::None);
    
    // Installation handling
    let mut current_installation_id = use_signal(|| Option::<String>::None);
    let mut installations = use_signal(|| props.installations.clone());

    // Get launcher configuration
    let launcher = match get_launcher(&config.read().launcher) {
        Ok(l) => Some(l),
        Err(e) => {
            error!("Failed to load launcher: {} - {}", config.read().launcher, e);
            None
        },
    };
    let has_launcher = launcher.is_some();

    // Load universal manifest with error handling
    let has_launcher_copy = has_launcher;

    let config_clone = config.clone();
    let manifest_error_clone = manifest_error.clone();
    let universal_manifest = use_resource(move || {
        let config = config_clone.clone();
        let mut manifest_error = manifest_error_clone.clone();
        async move {
            // Clone the launcher string
            let launcher_str = config.read().launcher.clone();
            
            // Now use the string value with get_launcher
            let launcher = match get_launcher(&launcher_str) {
                Ok(l) => Some(l),
                Err(_) => None,
            };
            
            let launcher_available = launcher.is_some();
            if !launcher_available {
                return None;
            }
            
            debug!("Loading universal manifest...");
            match crate::universal::load_universal_manifest(&CachedHttpClient::new(), 
                Some("https://raw.githubusercontent.com/Olinus10/installer-test/master/universal.json")).await {
                Ok(manifest) => {
                    debug!("Successfully loaded universal manifest: {}", manifest.name);
                    Some(manifest)
                },
                Err(e) => {
                    error!("Failed to load universal manifest: {}", e);
                    // Use spawn to update the signal
                    spawn(async move {
                        manifest_error.set(Some(e.clone()));
                    });
                    None
                }
            }
        }
    });
    
    // Load changelog
    let changelog = use_resource(move || async {
        match fetch_changelog("Olinus10/installer-test", &CachedHttpClient::new()).await {
            Ok(changelog) => {
                debug!("Successfully loaded changelog with {} entries", changelog.entries.len());
                Some(changelog)
            },
            Err(e) => {
                error!("Failed to load changelog: {}", e);
                None
            }
        }
    });

    // Modal context for popups
    let mut modal_context = use_context_provider(ModalContext::default);
    
    // Show error modal if error exists
    if let Some(e) = error_signal() {
        modal_context.open("Error", rsx! {
            p {
                "The installer encountered an error. If the problem persists, please report it in #📂modpack-issues on Discord."
            }
            textarea { class: "error-area", readonly: true, "{e}" }
        }, false, Some(move |_| error_signal.set(None)));
    }

    // Build CSS content
    let css_content = css
        .replace("<BG_COLOR>", "#320625")
        .replace("<BG_IMAGE>", "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png")
        .replace("<SECONDARY_FONT>", "\"HEADER_FONT\"")
        .replace("<PRIMARY_FONT>", "\"REGULAR_FONT\"");
    
    // Add custom category styles
let category_styles = include_str!("assets/category-styles.css");
let feature_styles = include_str!("assets/expanded-feature-styles.css");
let preset_styles = include_str!("assets/preset-styles.css");
let search_styles = include_str!("assets/search-results-styles.css");

let complete_css = format!("{}\n{}\n{}\n{}\n{}", 
    css_content, 
    category_styles, 
    feature_styles, 
    preset_styles, 
    search_styles
);

    // Create header component if needed
    let header_component = if !config.read().first_launch.unwrap_or(true) && has_launcher && !settings() {
        Some(rsx! {
            AppHeader {
                installations: installations.clone(),
                current_installation_id: current_installation_id.clone(),
                on_select_installation: move |id: String| {
                    if id == "new" {
                        // Special case for "new" installation
                        current_installation_id.set(Some(id));
                    } else {
                        // Normal case for existing installation
                        current_installation_id.set(Some(id));
                    }
                },
                on_go_home: move |_| {
                    current_installation_id.set(None);
                },
                on_open_settings: move |_| {
                    settings.set(true);
                }
            }
        })
    } else {
        None
    };

    // Determine what content to show
    let main_content = if settings() {
        // Settings screen
        rsx! {
            Settings {
                config,
                settings,
                config_path: props.config_path.clone(),
                error: error_signal.clone(),
                b64_id: URL_SAFE_NO_PAD.encode(props.modpack_source)
            }
        }
    } else if config.read().first_launch.unwrap_or(true) || !has_launcher {
        // Launcher selection for first launch
        rsx! {
            Launcher {
                config,
                config_path: props.config_path.clone(),
                error: error_signal.clone(),
                b64_id: URL_SAFE_NO_PAD.encode(props.modpack_source)
            }
        }
    } else if universal_manifest.read().is_none() && has_launcher {
        // Loading screen while universal manifest loads
        rsx! {
            div { class: "loading-container",
                div { class: "loading-spinner" }
                div { class: "loading-text", "Loading modpack information..." }
                p { class: "loading-info", "This may take a moment. Please wait..." }
            }
        }
    } else {
        // Main content based on current state
        if current_installation_id.read().is_none() {
            // Home page - show installations or welcome screen
            rsx! {
                HomePage {
                    installations,
                    error_signal: error_signal.clone(),
                    changelog: use_signal(|| changelog.read().as_ref().cloned().flatten()),
                    current_installation_id: current_installation_id.clone(),
                }
            }
        } else if current_installation_id.read().as_ref().map_or(false, |id| id == "new") {
            // New installation flow
            rsx! {
    SimplifiedInstallationWizard {
        onclose: move |_| {
            current_installation_id.set(None);
        },
        oncreate: move |new_installation: Installation| {  // Added type annotation here
            // Add the new installation to the list
            installations.with_mut(|list| {
                list.insert(0, new_installation.clone());
            });
            
            // Set the current installation to navigate to the installation page
            current_installation_id.set(Some(new_installation.id));
            
            // Explicitly return unit type to match expected return type
            ()
        }
    }
}
        } else {
            // Specific installation management page
            let back_handler = EventHandler::new(move |_| {
                current_installation_id.set(None);
            });
            
            let id = current_installation_id.read().as_ref().unwrap().clone();
            
            rsx! {
                InstallationManagementPage {
                    installation_id: id,
                    onback: back_handler,
                    installations: installations.clone()
                }
            }
        }
    };

    // Combine components for final render
    rsx! {
        div {
            style { {complete_css} }
            Modal {}

            BackgroundParticles {}
            
            // Show header when appropriate
            if let Some(header) = header_component {
                {header}
            }

            div { class: "main-container",
                {main_content}
            }
            
            // Only show footer on main pages
            if !settings() && current_installation_id.read().is_none() && 
               !config.read().first_launch.unwrap_or(true) && has_launcher {
                Footer {}
            }
            
            // Add manifest error display outside of the main container to ensure it appears on top
            if let Some(error) = manifest_error() {
                ManifestErrorDisplay {
                    error: error.message.clone(),
                    error_type: format!("{}", error.error_type),
                    file_name: error.file_name.clone(),
                    onclose: move |_| manifest_error.set(None),
                    onreport: move |_| {
                        let _ = open_url("https://discord.com/channels/778965021656743966/1234506784626970684");
                    }
                }
            }
        }
    }
}
