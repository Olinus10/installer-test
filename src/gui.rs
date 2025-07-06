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
use std::collections::{BTreeMap, HashMap};
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
use crate::installation::delete_installation;
use crate::preset::find_preset_by_id;

mod modal;

// Font constants
const HEADER_FONT: &str = "\"HEADER_FONT\"";
const REGULAR_FONT: &str = "\"REGULAR_FONT\"";
const ICON_BYTES: &[u8] = include_bytes!("assets/icon.png");

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
            // Basic floating particles - simplified to only circles
            for i in 0..30 {
                {
                    let size = 2 + (i % 6); // Size variation
                    let delay = (i as f32) * 0.4; // Staggered delays
                    let duration = 8.0 + (i % 10) as f32; // Duration variation
                    let left = 5 + (i * 3) % 90; // Better distribution
                    
                    // Simplified particle classes - only basic and glow
                    let particle_class = match i % 3 {
                        0 => "particle glow",
                        1 => "particle subtle",
                        _ => "particle",
                    };
                    
                    // Simpler animations - just float
                    let animation = match i % 2 {
                        0 => "float",
                        _ => "float-horizontal",
                    };
                    
                    rsx! {
                        div {
                            class: "{particle_class}",
                            style: "width: {size}px; height: {size}px; left: {left}%; 
                                bottom: -50px; opacity: {0.2 + (i % 3) as f32 * 0.15}; 
                                animation: {animation} {duration}s ease-in-out infinite {delay}s;"
                        }
                    }
                }
            }
            
            // Add some larger ambient orbs
            for i in 30..40 {
                {
                    let size = 12 + (i % 8);
                    let delay = (i as f32) * 1.5;
                    let duration = 20.0 + (i % 12) as f32;
                    let left = 10 + (i * 8) % 80;
                    let opacity = 0.1 + (i % 2) as f32 * 0.1;
                    
                    rsx! {
                        div {
                            class: "particle glow",
                            style: "width: {size}px; height: {size}px; left: {left}%; 
                                bottom: -80px; opacity: {opacity}; 
                                animation: float {duration}s ease-in-out infinite {delay}s;"
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

#[component]
fn ChangelogSection(changelog: Option<ChangelogData>) -> Element {
    let mut show_all = use_signal(|| false);
    
    match changelog {
        Some(changelog_data) if !changelog_data.entries.is_empty() => {
            let display_count = if *show_all.read() { changelog_data.entries.len() } else { 3 };
            
            rsx! {
                div { class: "changelog-container",
                    div { class: "section-divider with-title", 
                        span { class: "divider-title", "LATEST CHANGES" }
                    }
                    
                    div { class: "changelog-entries",
                        for (index, entry) in changelog_data.entries.iter().enumerate().take(display_count) {
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
                                
                                if index < display_count - 1 {
                                    div { class: "entry-divider" }
                                }
                            }
                        }
                        
                        if changelog_data.entries.len() > 3 {
                            div { class: "view-all-changes",
                                button { 
                                    class: "view-all-button",
                                    onclick: move |_| {
                                        let current_state = *show_all.read();
                                        show_all.set(!current_state);
                                    },
                                    if *show_all.read() {
                                        "Show Less"
                                    } else {
                                        {format!("View {} More Changes", changelog_data.entries.len() - 3)}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        _ => {
            rsx! { Fragment {} }
        }
    }
}

#[component]
fn FloatingLogo(onclick: EventHandler<()>) -> Element {
    let icon_base64 = {
        use base64::{Engine, engine::general_purpose::STANDARD};
        STANDARD.encode(include_bytes!("assets/icon.png"))
    };
    
    rsx! {
        div { 
            class: "floating-logo",
            onclick: move |_| onclick.call(()),
            
            img { 
                src: "data:image/png;base64,{icon_base64}",
                alt: "Wynncraft Overhaul Logo"
            }
        }
    }
}

#[component]
fn HomeFloatingHeader() -> Element {
    rsx! {
        div { class: "home-floating-header",
            h1 { class: "home-header-title", "Majestic Overhaul" }
        }
    }
}

#[component]
fn InstallationFloatingHeader(
    installation_name: String,
    minecraft_version: String,
    loader_info: String,
    active_tab: Signal<String>,
    on_tab_change: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "installation-floating-header",
            div { class: "installation-header-info",
                h1 { class: "installation-header-title", "{installation_name}" }
                div { class: "installation-header-meta",
                    span { class: "installation-meta-chip", "Minecraft {minecraft_version}" }
                    span { class: "installation-meta-chip", "{loader_info}" }
                }
            }
            
            div { class: "installation-header-tabs",
                button { 
                    class: if *active_tab.read() == "features" { "installation-tab active" } else { "installation-tab" },
                    onclick: move |_| on_tab_change.call("features".to_string()),
                    "Features"
                }
                button { 
                    class: if *active_tab.read() == "performance" { "installation-tab active" } else { "installation-tab" },
                    onclick: move |_| on_tab_change.call("performance".to_string()),
                    "Performance"
                }
                button { 
                    class: if *active_tab.read() == "settings" { "installation-tab active" } else { "installation-tab" },
                    onclick: move |_| on_tab_change.call("settings".to_string()),
                    "Settings"
                }
            }
        }
    }
}

#[component]
fn FloatingLaunchButton(
    is_installed: bool,
    is_installing: bool,
    onclick: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            class: "floating-launch-button",
            disabled: !is_installed || is_installing,
            onclick: move |_| onclick.call(()),
            
            if is_installed {
                "LAUNCH"
            } else {
                "INSTALL FIRST"
            }
        }
    }
}

#[component]
fn FloatingBackButton(onclick: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: "floating-back-button",
            onclick: move |_| onclick.call(()),
            "←"
        }
    }
}

#[component]
fn FloatingInstallButton(
    button_text: String,
    button_class: String,
    disabled: bool,
    onclick: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            class: "floating-install-button {button_class}",
            disabled: disabled,
            onclick: move |_| onclick.call(()),
            "{button_text}"
        }
    }
}

#[component]
fn ScrollIndicator() -> Element {
    rsx! {
        div { class: "scroll-indicator",
            span { "Scroll down for more info" }
            span { class: "scroll-indicator-arrow", "↓" }
        }
    }
}

#[component]
fn FloatingDiscordButton() -> Element {
    rsx! {
        a { 
            class: "floating-discord-button",
            href: "https://discord.gg/olinus-corner-778965021656743966",
            target: "_blank",
            rel: "noopener noreferrer",
            
            svg {
                width: "24",
                height: "24",
                view_box: "0 0 24 24",
                fill: "currentColor",
                
                path {
                    d: "M19.54 0c1.356 0 2.46 1.104 2.46 2.472v21.528l-2.58-2.28-1.452-1.344-1.536-1.428.636 2.22h-13.608c-1.356 0-2.46-1.104-2.46-2.472v-16.224c0-1.368 1.104-2.472 2.46-2.472h16.08zm-4.632 15.672c2.652-.084 3.672-1.824 3.672-1.824 0-3.864-1.728-6.996-1.728-6.996-1.728-1.296-3.372-1.26-3.372-1.26l-.168.192c2.04.624 2.988 1.524 2.988 1.524-1.248-.684-2.472-1.02-3.612-1.152-.864-.096-1.692-.072-2.424.024l-.204.024c-.42.036-1.44.192-2.724.756-.444.204-.708.348-.708.348s.996-.948 3.156-1.572l-.12-.144s-1.644-.036-3.372 1.26c0 0-1.728 3.132-1.728 6.996 0 0 1.008 1.74 3.66 1.824 0 0 .444-.54.804-.996-1.524-.456-2.1-1.416-2.1-1.416l.336.204.048.036.047.027.014.006.047.027c.3.168.6.3.876.408.492.192 1.08.384 1.764.516.9.168 1.956.228 3.108.012.564-.096 1.14-.264 1.74-.516.42-.156.888-.384 1.38-.708 0 0-.6.984-2.172 1.428.36.456.792.972.792.972zm-5.58-5.604c-.684 0-1.224.6-1.224 1.332 0 .732.552 1.332 1.224 1.332.684 0 1.224-.6 1.224-1.332.012-.732-.54-1.332-1.224-1.332zm4.38 0c-.684 0-1.224.6-1.224 1.332 0 .732.552 1.332 1.224 1.332.684 0 1.224-.6 1.224-1.332 0-.732-.54-1.332-1.224-1.332z"
                }
            }
            
            span { "Discord" }
        }
    }
}

#[component]
fn FloatingFooter(
    is_home_page: bool,
    installation_info: Option<InstallationFooterInfo>,
    on_back: Option<EventHandler<()>>,
    on_install: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        footer { class: "floating-footer",
            if is_home_page {
                // Home page footer - just stats or minimal content
                div { class: "footer-home-content",
                    // Maybe show some stats or leave mostly empty
                    div { class: "footer-stats",
                        span { "200+ FPS • 100+ Mods • 20K+ Downloads" }
                    }
                }
            } else if let Some(info) = installation_info {
                // Installation page footer content
                div { class: "footer-installation-content",
                    // Back button (blue position in footer)
                    if let Some(back_handler) = on_back {
                        button {
                            class: "footer-back-button",
                            onclick: move |_| back_handler.call(()),
                            "← Back"
                        }
                    }
                    
                    // Installation info in center
                    div { class: "footer-installation-info",
                        h3 { class: "footer-installation-name", "{info.name}" }
                        div { class: "footer-installation-details",
                            span { "Minecraft {info.minecraft_version}" }
                            span { "{info.loader_type} {info.loader_version}" }
                            span { "{info.memory_allocation} MB" }
                        }
                    }
                    
                    // Install/Update button on the right
                    if let Some(install_handler) = on_install {
                        button {
                            class: if info.needs_update {
                                "footer-install-button update-button"
                            } else {
                                "footer-install-button"
                            },
                            disabled: info.is_up_to_date && !info.has_changes,
                            onclick: move |_| install_handler.call(()),
                            
                            if !info.installed {
                                "INSTALL"
                            } else if info.needs_update {
                                "UPDATE"
                            } else if info.has_changes {
                                "APPLY CHANGES"
                            } else {
                                "UP TO DATE"
                            }
                        }
                    }
                }
            }
        }
    }
}

// Helper struct for footer information
#[derive(Clone, Debug, PartialEq)]
pub struct InstallationFooterInfo {
    pub name: String,
    pub minecraft_version: String,
    pub loader_type: String,
    pub loader_version: String,
    pub memory_allocation: i32,
    pub installed: bool,
    pub needs_update: bool,
    pub has_changes: bool,
    pub is_up_to_date: bool,
}

// Update your main app layout
#[component]
fn ModernAppLayout(
    is_home_page: bool,
    current_installation: Option<Installation>,
    active_tab: Signal<String>,
    children: Element,
    on_go_home: EventHandler<()>,
    on_tab_change: EventHandler<String>,
    on_launch: Option<EventHandler<()>>,
    on_back: Option<EventHandler<()>>,
    on_install: Option<EventHandler<()>>,
    has_changes: bool,
    is_installing: bool,
    install_button_text: String,
    install_button_class: String,
    install_button_disabled: bool,
) -> Element {
    // State for scroll indicator
    let mut show_scroll_indicator = use_signal(|| false);
    
    // Effect to check if content is scrollable
    use_effect(move || {
        // This would need to be implemented with JavaScript to check scroll height
        // For now, we'll show it by default and hide after user scrolls
        show_scroll_indicator.set(true);
        
        // Auto-hide after 5 seconds
        spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            show_scroll_indicator.set(false);
        });
    });
    
    rsx! {
        div { class: "modern-app-layout",
            // Always show floating logo
            FloatingLogo {
                onclick: on_go_home
            }
            
            // Show appropriate header
            if is_home_page {
                HomeFloatingHeader {}
                FloatingDiscordButton {}
            } else if let Some(installation) = &current_installation {
                InstallationFloatingHeader {
                    installation_name: installation.name.clone(),
                    minecraft_version: installation.minecraft_version.clone(),
                    loader_info: format!("{} {}", installation.loader_type, installation.loader_version),
                    active_tab: active_tab,
                    on_tab_change: on_tab_change,
                }
                
                if let Some(launch_handler) = on_launch {
                    FloatingLaunchButton {
                        is_installed: installation.installed,
                        is_installing: is_installing,
                        onclick: launch_handler,
                    }
                }
            }
            
            // Scrollable content area
            div { 
                class: if is_home_page {
                    "page-content-area home-page"
                } else {
                    "page-content-area"
                },
                onscroll: move |_| {
                    // Hide scroll indicator when user starts scrolling
                    show_scroll_indicator.set(false);
                },
                {children}
            }
            
            // Show scroll indicator if there's more content
            if *show_scroll_indicator.read() && !is_home_page {
                ScrollIndicator {}
            }    
            
            // Floating back button (only on installation pages)
            if !is_home_page {
                if let Some(back_handler) = on_back {
                    FloatingBackButton {
                        onclick: back_handler
                    }
                }
                
                // Floating install button (only on installation pages)
                if let Some(install_handler) = on_install {
                    FloatingInstallButton {
                        button_text: install_button_text,
                        button_class: install_button_class,
                        disabled: install_button_disabled,
                        onclick: install_handler,
                    }
                }
            }
            
            // Copyright in bottom corner - always visible
            div { class: "floating-copyright",
                "© 2023-2025 Majestic Overhaul. CC BY-NC-SA 4.0."
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
    current_installation_id: Signal<Option<String>>,
) -> Element {
    // State for the installation creation dialog
    let mut show_creation_dialog = use_signal(|| false);
    
    // Check if this is the first time (no installations)
    let has_installations = !installations().is_empty();
    let latest_installation = installations().first().cloned();
    
    rsx! {
        ModernAppLayout {
            is_home_page: true,
            current_installation: None,
            active_tab: use_signal(|| "home".to_string()), // Dummy tab signal for home page
            has_changes: false,
            is_installing: false,
            install_button_text: "".to_string(), // Not used on home page
            install_button_class: "".to_string(), // Not used on home page
            install_button_disabled: true, // Not used on home page
            on_go_home: EventHandler::new(move |_: ()| {
                // Already on home, do nothing
            }),
            on_tab_change: EventHandler::new(move |_tab: String| {
                // No tab changes on home page
            }),
            on_launch: None, // No launch button on home page
            on_back: None, // No back button on home page
            on_install: None, // No install button on home page
            
            // Home page content
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

                // Update notification if available
                if let Some(installation) = latest_installation {
                    if installation.update_available {
                        div { class: "update-notification",
                            "Update available for {installation.name}!"
                            button {
                                class: "update-button",
                                onclick: move |_| {
                                    current_installation_id.set(Some(installation.id.clone()));
                                },
                                "Update Now"
                            }
                        }
                    }
                }
                
                if has_installations {
                    // Regular home page with installations
                    
                    // Statistics display
                    StatisticsDisplay {}
                    
                    // Section divider for installations
                    div { class: "section-divider with-title", 
                        span { class: "divider-title", "YOUR INSTALLATIONS" }
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
                        h1 { "Welcome to the MAJESTIC OVERHAUL" }
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
                        oncreate: move |new_installation: Installation| {
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
    span { 
        class: "update-badge", 
if installation.preset_update_available && !installation.update_available {
    "Preset Update"
} else if !installation.preset_update_available && installation.update_available {
    "Modpack Update"
} else {
    "Updates Available"
}
    }
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
    
    // Character limit for installation names
    const MAX_NAME_LENGTH: usize = 25;
    
    // Suggested names based on existing installations count
    let installations = use_context::<AppProps>().installations;
    let installation_count = installations.len() + 1;
    let suggested_names = vec![
        format!("Overhaul {}", installation_count),
        format!("Wynncraft {}", installation_count),
        format!("Adventure {}", installation_count),
        "My Overhaul".to_string(),
        "Custom Build".to_string(),
    ];
    
    // Set default name based on count
    use_effect(move || {
        if *name.read() == "My Wynncraft Installation" {
            name.set(format!("Overhaul {}", installation_count));
        }
    });
    
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
        
        // Validate name length
        let installation_name = name.read().trim().to_string();
        if installation_name.is_empty() {
            installation_error.set(Some("Installation name cannot be empty.".to_string()));
            return;
        }
        
        if installation_name.len() > MAX_NAME_LENGTH {
            installation_error.set(Some(format!("Installation name cannot exceed {} characters.", MAX_NAME_LENGTH)));
            return;
        }
        
        // Get the universal manifest for Minecraft version and loader information
        if let Some(unwrapped_manifest) = universal_manifest.read().as_ref().and_then(|opt| opt.as_ref()) {
            let minecraft_version = unwrapped_manifest.minecraft_version.clone();
            let loader_type = unwrapped_manifest.loader.r#type.clone();
            let loader_version = unwrapped_manifest.loader.version.clone();
            
            // Create a basic installation
let installation = Installation::new_custom(
    installation_name.clone(),  // Use the validated name
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
                // Header with close button in corner
                div { class: "wizard-header",
                    h2 { "Create New Installation" }
                    button { 
                        class: "close-button",
                        onclick: move |_| props.onclose.call(()),
                        "×"
                    }
                }
                
                // Main content
                div { class: "wizard-content",
                    // Error notification if any
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
                    
                    // Name section
                    div { class: "wizard-section",
                        h3 { "Installation Name" }
                        div { class: "form-group",
                            label { r#for: "installation-name", "Name your installation:" }
                            input {
                                id: "installation-name",
                                r#type: "text",
                                value: "{name}",
                                maxlength: "{MAX_NAME_LENGTH}",
                                oninput: move |evt| {
                                    let new_value = evt.value().clone();
                                    if new_value.len() <= MAX_NAME_LENGTH {
                                        name.set(new_value);
                                    }
                                },
                                placeholder: "e.g. My Wynncraft Adventure"
                            }
                            
                            // Character counter
                            div { class: "character-counter",
                                style: if name.read().len() > MAX_NAME_LENGTH - 5 { 
                                    "color: #ff9d93;" 
                                } else { 
                                    "color: rgba(255, 255, 255, 0.6);" 
                                },
                                "{name.read().len()}/{MAX_NAME_LENGTH}"
                            }
                        }
                        
                        // Suggested names
                        div { class: "suggested-names",
                            span { class: "suggestion-label", "Quick suggestions:" }
                            div { class: "suggestion-chips",
                                for suggestion in suggested_names {
                                    button {
                                        class: "suggestion-chip",
                                        r#type: "button",
                                        onclick: move |_| name.set(suggestion.clone()),
                                        "{suggestion}"
                                    }
                                }
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
                
                // Footer with buttons
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

// Installation management page - SIMPLIFIED VERSION WITHOUT COMPLEX HEADER
// Complete InstallationManagementPageWithLayout component
// Replace your existing InstallationManagementPage with this

#[component]
pub fn InstallationManagementPageWithLayout(
    installation_id: String,
    onback: EventHandler<()>,
    installations: Signal<Vec<Installation>>,
) -> Element {
    // State for the current tab
    let mut active_tab = use_signal(|| "features".to_string());

    // Load the installation data
    let installation_result = use_memo(move || {
        crate::installation::load_installation(&installation_id)
    });

    // Installation status signals
    let mut is_installing = use_signal(|| false);
    let mut installation_error = use_signal(|| Option::<String>::None);
    
    // Progress tracking signals
    let mut installation_progress = use_signal(|| 0i64);
    let mut installation_total = use_signal(|| 0i64);
    let mut installation_status = use_signal(|| String::new());
    
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
    
    // Create the installation state signal AFTER we have the installation
    let mut installation_state = use_signal(|| installation.clone());
    
    // Clone needed values to avoid partial moves
    let installation_id_for_delete = installation.id.clone();
    let installation_id_for_launch = installation.id.clone();
    let installation_for_update = installation.clone();
    
    // State for modification tracking
    let mut has_changes = use_signal(|| false);
    let enabled_features = use_signal(|| installation.enabled_features.clone());
    let memory_allocation = use_signal(|| installation.memory_allocation);
    let java_args = use_signal(|| installation.java_args.clone());
    let selected_preset = use_signal(|| Option::<String>::None);
    
    // Filter text for feature search
    let filter_text = use_signal(|| String::new());
    
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
        let memory_for_effect = memory_allocation.clone();
        let java_for_effect = java_args.clone();
        let original_features = installation.enabled_features.clone();
        let original_memory = installation.memory_allocation;
        let original_java = installation.java_args.clone();
        let mut has_changes_copy = has_changes.clone();
        
        move || {
            let features_changed = enabled_features_for_effect.read().clone() != original_features;
            let memory_changed = *memory_for_effect.read() != original_memory;
            let java_changed = java_for_effect.read().clone() != original_java;
            
            let any_changes = features_changed || memory_changed || java_changed;
            has_changes_copy.set(any_changes);
        }
    });
    
    // Handle install/update with progress tracking
    let handle_install_update = EventHandler::new({
        let mut is_installing = is_installing.clone();
        let mut installation_error = installation_error.clone();
        let mut installation_progress = installation_progress.clone();
        let mut installation_total = installation_total.clone();
        let mut installation_status = installation_status.clone();
        let mut has_changes = has_changes.clone();
        let mut installations = installations.clone();
        let mut installation_state = installation_state.clone();
        let enabled_features = enabled_features.clone();
        let memory_allocation = memory_allocation.clone();
        let java_args = java_args.clone();
        let installation_for_update = installation_for_update.clone();
        
        move |_: ()| {
            is_installing.set(true);
            let mut installation_clone = installation_for_update.clone();
            
            // Update settings
            installation_clone.enabled_features = enabled_features.read().clone();
            installation_clone.memory_allocation = *memory_allocation.read();
            installation_clone.java_args = java_args.read().clone();
            installation_clone.modified = true;
            
            let http_client = crate::CachedHttpClient::new();
            let mut installation_error_clone = installation_error.clone();
            let mut progress = installation_progress.clone();
            let mut total = installation_total.clone();
            let mut status = installation_status.clone();
            let mut is_installing_clone = is_installing.clone();
            let mut has_changes_clone = has_changes.clone();
            let mut installations = installations.clone();
            let mut installation_state = installation_state.clone();
            let installation_id = installation_clone.id.clone();

            spawn(async move {
                // Calculate total items
                match crate::universal::load_universal_manifest(&http_client, None).await {
                    Ok(manifest) => {
                        let total_items = manifest.mods.len() + manifest.shaderpacks.len() + 
                                         manifest.resourcepacks.len() + manifest.include.len();
                        total.set(total_items as i64);
                        progress.set(0);
                        status.set("Preparing installation...".to_string());
                        
                        // Create a progress callback
                        let progress_callback = move || {
                            progress.with_mut(|p| *p += 1);
                            let current = *progress.read();
                            let total_val = *total.read();
                            status.set(format!("Installing... {}/{}", current, total_val));
                        };
                        
                        match installation_clone.install_or_update_with_progress(&http_client, progress_callback).await {
                            Ok(_) => {
                                // Mark as installed
                                installation_clone.installed = true;
                                installation_clone.update_available = false;
                                installation_clone.modified = false;
                                
                                // Save the installation
                                if let Err(e) = installation_clone.save() {
                                    error!("Failed to save installation: {}", e);
                                    installation_error_clone.set(Some(format!("Failed to save installation: {}", e)));
                                } else {
                                    // Update installation state
                                    installation_state.set(installation_clone.clone());
                                    
                                    // Update the installations list
                                    installations.with_mut(|list| {
                                        if let Some(index) = list.iter().position(|i| i.id == installation_id) {
                                            list[index] = installation_clone;
                                        }
                                    });
                                    
                                    // Clear modification flags
                                    has_changes_clone.set(false);
                                    
                                    // Stop showing progress after a brief delay
                                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                }
                            },
                            Err(e) => {
                                error!("Installation failed: {}", e);
                                installation_error_clone.set(Some(format!("Installation failed: {}", e)));
                            }
                        }
                    },
                    Err(e) => {
                        error!("Failed to load manifest: {}", e);
                        installation_error_clone.set(Some(format!("Failed to load manifest: {}", e)));
                    }
                }
                
                // Always stop installing state
                is_installing_clone.set(false);
            });
        }
    });
    
    // Handle launch
    let handle_launch = EventHandler::new({
        let mut installation_error_clone = installation_error.clone();
        let installation_id = installation_id_for_launch.clone();
        
        move |_: ()| {
            let mut installation_error_clone = installation_error_clone.clone();
            let installation_id = installation_id.clone();
            
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
        }
    });

    // Button state logic
    let (button_text, button_class, button_disabled) = {
        let current_installation = installation_state.read();
        let installed = current_installation.installed;
        let update_available = current_installation.update_available;
        let has_changes_val = *has_changes.read();
        let is_installing_val = *is_installing.read();
        
        debug!("Button state check: installed={}, update_available={}, has_changes={}, is_installing={}", 
               installed, update_available, has_changes_val, is_installing_val);
        
        if is_installing_val {
            ("INSTALLING...".to_string(), "installing".to_string(), true)
        } else if !installed {
            ("INSTALL".to_string(), "".to_string(), false)
        } else if update_available {
            ("UPDATE".to_string(), "update-button".to_string(), false)
        } else if has_changes_val {
            ("MODIFY".to_string(), "".to_string(), false)
        } else {
            ("UPDATED".to_string(), "up-to-date".to_string(), true)
        }
    };

    // Show progress view if installing
    if *is_installing.read() {
        return rsx! {
            ProgressView {
                value: *installation_progress.read(),
                max: *installation_total.read(),
                status: installation_status.read().clone(),
                title: format!("Installing {}", installation.name)
            }
        };
    }

    // Main render with ModernAppLayout
    rsx! {
        div { class: "installation-management-container",
            // Error display
            if let Some(error) = &*installation_error.read() {
                ErrorNotification {
                    message: error.clone(),
                    on_close: move |_| {
                        installation_error.set(None);
                    }
                }
            }

            ModernAppLayout {
                is_home_page: false,
                current_installation: Some(installation_state.read().clone()),
                active_tab: active_tab,
                has_changes: *has_changes.read(),
                is_installing: *is_installing.read(),
                install_button_text: button_text,
                install_button_class: button_class,
                install_button_disabled: button_disabled,
                on_go_home: EventHandler::new(move |_: ()| {
                    onback.call(());
                }),
                on_tab_change: EventHandler::new(move |tab: String| {
                    active_tab.set(tab);
                }),
                on_launch: Some(handle_launch),
                on_back: Some(EventHandler::new(move |_: ()| {
                    onback.call(());
                })),
                on_install: Some(handle_install_update),
                
                // Main content area based on active tab
                div { class: "installation-main-content",
                    match active_tab.read().as_str() {
                        "features" => {
                            rsx! {
                                FeaturesTab {
                                    universal_manifest: universal_manifest.read().clone().flatten(),
                                    presets: presets.read().clone().unwrap_or_default(),
                                    enabled_features: enabled_features,
                                    selected_preset: selected_preset,
                                    filter_text: filter_text,
                                    installation_id: installation.id.clone(),
                                }
                            }
                        },
                        "performance" => {
                            rsx! {
                                PerformanceTab {
                                    memory_allocation: memory_allocation,
                                    java_args: java_args,
                                    installation_id: installation.id.clone()
                                }
                            }
                        },
                        "settings" => {
                            rsx! {
                                SettingsTab {
                                    installation: installation.clone(),
                                    installation_id: installation_id_for_delete.clone(),
                                    ondelete: move |_| {
                                        // Handle delete functionality - remove from installations list
                                        let id_to_delete = installation_id_for_delete.clone();
                                        installations.with_mut(|list| {
                                            list.retain(|inst| inst.id != id_to_delete);
                                        });
                                        // Navigate back to home
                                        onback.call(());
                                    },
                                    onupdate: move |updated_installation: Installation| {
                                        // Update the installation data in the list
                                        installations.with_mut(|list| {
                                            if let Some(index) = list.iter().position(|i| i.id == updated_installation.id) {
                                                list[index] = updated_installation.clone();
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
    
    // Add completion detection
    let is_complete = percentage >= 100;
    
    // Auto-hide progress after completion with delay
    let mut auto_close_timer = use_signal(|| 0);
    
    use_effect({
        let mut auto_close_timer = auto_close_timer.clone();
        move || {
            if is_complete {
                spawn(async move {
                    // Wait 2 seconds after completion, then signal to close
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    auto_close_timer.set(1);
                });
            }
        }
    });
    
    // Helpful messages to display during download
    let helpful_messages = vec![
        "Large files may take a while to download...",
        "Download speed depends on your internet connection",
        "Please be patient while files are being downloaded",
        "Some files can be over 1GB in size",
        "The installer is still working - don't close it!",
    ];
    
    // Message index for cycling through helpful messages
    let mut message_index = use_signal(|| 0);
    
    // Ensure we show completion state
    let (current_step, _step_label) = if percentage >= 100 {
        ("complete", "Complete")
    } else if percentage >= 90 {
        ("finish", "Finishing")
    } else if percentage >= 60 {
        ("configure", "Configuring")
    } else if percentage >= 30 {
        ("extract", "Extracting")
    } else if percentage > 0 {
        ("download", "Downloading")
    } else {
        ("prepare", "Preparing")
    };
    
    let steps = vec![
        ("prepare", "Prepare"),
        ("download", "Download"),
        ("extract", "Extract"),
        ("configure", "Configure"),
        ("finish", "Finish"),
        ("complete", "Complete"), // Add completion step
    ];
    
    // Find current step index
    let active_step_index = steps.iter().position(|(id, _)| id == &current_step).unwrap_or(0);
    
    // Show final status when complete
    let display_status = if percentage >= 100 {
        "Installation completed successfully!".to_string()
    } else {
        status
    };
    
    rsx! {
        div { 
            class: "progress-container",
            "data-complete": if is_complete { "true" } else { "false" },
            "data-value": "{value}",
            "data-max": "{max}",
            "data-step": "{current_step}",
            
            div { class: "progress-header",
                h1 { "{title}" }
                div { class: "progress-subtitle", 
                    if percentage >= 100 {
                        "Installation Complete!"
                    } else {
                        "Installation in progress..."
                    }
                }
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
                
                p { class: "progress-status", "{display_status}" }
                
                // Add helpful message during download
                if percentage > 0 && percentage < 100 && current_step == "download" {
                    p { 
                        class: "progress-helpful-message",
                        style: "color: rgba(255, 255, 255, 0.7); font-size: 0.9rem; margin-top: 10px; font-style: italic;",
                        "{helpful_messages[*message_index.read()]}"
                    }
                }
                
                // Add completion button
                if is_complete && *auto_close_timer.read() > 0 {
                    div { class: "completion-overlay",
                        "Installation completed successfully!"
                        button {
                            onclick: move |_| {
                                // Signal completion to parent
                                debug!("Progress completion acknowledged");
                            },
                            "Continue"
                        }
                    }
                }
            }
        }
    }
}

// Additional components for compatibility
#[derive(PartialEq, Props, Clone)]
struct SettingsProps {
    config: Signal<crate::Config>,
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
            h1 { class: "settings-title", "Launcher Settings" }
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
                    label { class: "setting-label", "Minecraft Launcher:" }
                    select {
                        name: "launcher-select",
                        id: "launcher-select",
                        form: "settings",
                        class: "setting-select",
                        if crate::get_minecraft_folder().is_dir() {
                            option { value: "vanilla", selected: vanilla, "Vanilla Launcher" }
                        }
                        if crate::get_multimc_folder("MultiMC").is_ok() {
                            option { value: "multimc-MultiMC", selected: multimc, "MultiMC" }
                        }
                        if crate::get_multimc_folder("PrismLauncher").is_ok() {
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
                        value: "Save Changes",
                        class: "primary-button",
                        id: "save"
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct LauncherProps {
    config: Signal<crate::Config>,
    config_path: PathBuf,
    error: Signal<Option<String>>,
    b64_id: String,
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
    let has_supported_launcher = crate::get_minecraft_folder().is_dir()
        || crate::get_multimc_folder("MultiMC").is_ok()
        || crate::get_multimc_folder("PrismLauncher").is_ok();
        
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
                            if crate::get_minecraft_folder().is_dir() {
                                option { value: "vanilla", selected: vanilla, "Vanilla" }
                            }
                            if crate::get_multimc_folder("MultiMC").is_ok() {
                                option {
                                    value: "multimc-MultiMC",
                                    selected: multimc,
                                    "MultiMC"
                                }
                            }
                            if crate::get_multimc_folder("PrismLauncher").is_ok() {
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

// State management structure for installations
#[derive(Clone, Debug, Default)]
pub struct InstallationState {
    pub enabled_features: Vec<String>,
    pub memory_allocation: i32,
    pub java_args: String,
    pub has_changes: bool,
    
    // Original values for comparison
    pub original_features: Vec<String>,
    pub original_memory: i32,
    pub original_java_args: String,
}

impl InstallationState {
    pub fn from_installation(installation: &Installation) -> Self {
        Self {
            enabled_features: installation.enabled_features.clone(),
            memory_allocation: installation.memory_allocation,
            java_args: installation.java_args.clone(),
            has_changes: false,
            
            original_features: installation.enabled_features.clone(),
            original_memory: installation.memory_allocation,
            original_java_args: installation.java_args.clone(),
        }
    }
    
    pub fn has_feature_changes(&self) -> bool {
        self.enabled_features != self.original_features
    }
    
    pub fn has_performance_changes(&self) -> bool {
        self.memory_allocation != self.original_memory || 
        self.java_args != self.original_java_args
    }
    
    pub fn reset_to_original(&mut self) {
        self.enabled_features = self.original_features.clone();
        self.memory_allocation = self.original_memory;
        self.java_args = self.original_java_args.clone();
        self.has_changes = false;
    }
    
    pub fn save_as_original(&mut self) {
        self.original_features = self.enabled_features.clone();
        self.original_memory = self.memory_allocation;
        self.original_java_args = self.java_args.clone();
        self.has_changes = false;
    }
}

#[derive(Debug, Clone)]
pub struct AppProps {
    pub branches: Vec<GithubBranch>,
    pub modpack_source: String,
    pub config: crate::Config,
    pub config_path: PathBuf,
    pub installations: Vec<Installation>,
}

// Fixed app function with modern layout
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
    let config_clone = config.clone();
    let manifest_error_clone = manifest_error.clone();
    let universal_manifest = use_resource(move || {
        let config = config_clone.clone();
        let mut manifest_error = manifest_error_clone.clone();
        async move {
            let launcher_str = config.read().launcher.clone();
            let launcher = match get_launcher(&launcher_str) {
                Ok(l) => Some(l),
                Err(_) => None,
            };
            
            let launcher_available = launcher.is_some();
            if !launcher_available {
                return None;
            }
            
            debug!("Loading universal manifest...");
            match crate::universal::load_universal_manifest(&CachedHttpClient::new(), None).await {
                Ok(manifest) => {
                    debug!("Successfully loaded universal manifest: {}", manifest.name);
                    Some(manifest)
                },
                Err(e) => {
                    error!("Failed to load universal manifest: {}", e);
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
        match crate::changelog::fetch_changelog("Olinus10/installer-test/master", &CachedHttpClient::new()).await {
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

    let mut changelog_signal = use_signal(|| None::<ChangelogData>);
    use_effect(move || {
        if let Some(Some(changelog_data)) = changelog.read().as_ref() {
            changelog_signal.set(Some(changelog_data.clone()));
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
    
    // Add all the CSS files including the new modern layout
    let category_styles = include_str!("assets/category-styles.css");
    let feature_styles = include_str!("assets/expanded-feature-styles.css");
    let preset_styles = include_str!("assets/preset-styles.css");
    let search_styles = include_str!("assets/search-results-styles.css");
    let modal_styles = include_str!("assets/modal-styles.css");
    let modern_layout = include_str!("assets/modern_layout.css"); // NEW: Your modern layout CSS
    
    // Combine all CSS files
    let complete_css = format!("{}\n{}\n{}\n{}\n{}\n{}\n{}", 
        css_content, 
        category_styles, 
        feature_styles, 
        preset_styles, 
        search_styles,
        modal_styles,
        modern_layout
    );

    // Determine current state
    let is_home_page = current_installation_id.read().is_none();

    // Determine main content
    let main_content = if settings() {
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
        rsx! {
            Launcher {
                config,
                config_path: props.config_path.clone(),
                error: error_signal.clone(),
                b64_id: URL_SAFE_NO_PAD.encode(props.modpack_source)
            }
        }
    } else if universal_manifest.read().is_none() && has_launcher {
        rsx! {
            div { class: "loading-container",
                div { class: "loading-spinner" }
                div { class: "loading-text", "Loading modpack information..." }
            }
        }
    } else if is_home_page {
        rsx! {
            HomePage {
                installations,
                error_signal: error_signal.clone(),
                changelog: changelog_signal,
                current_installation_id: current_installation_id.clone(),
            }
        }
    } else if current_installation_id.read().as_ref().map_or(false, |id| id == "new") {
        rsx! {
            SimplifiedInstallationWizard {
                onclose: move |_| {
                    current_installation_id.set(None);
                },
                oncreate: move |new_installation: Installation| {
                    installations.with_mut(|list| {
                        list.insert(0, new_installation.clone());
                    });
                    current_installation_id.set(Some(new_installation.id));
                }
            }
        }
    } else {
        // Installation management content using the new layout
        let id = current_installation_id.read().as_ref().unwrap().clone();
        let back_handler_for_mgmt = EventHandler::new(move |_: ()| {
            current_installation_id.set(None);
        });
        
        rsx! {
            InstallationManagementPageWithLayout {
                installation_id: id,
                onback: back_handler_for_mgmt,
                installations: installations.clone()
            }
        }
    };

    // Final render
    rsx! {
        div {
            style { {complete_css} }
            Modal {}
            BackgroundParticles {}
            
            // Show special screens (settings, launcher) without modern layout
            if settings() || config.read().first_launch.unwrap_or(true) || !has_launcher {
                div { class: "main-container",
                    {main_content}
                }
            } else {
                // Use the new modern layout for normal operation
                {main_content}
            }
            
            // Keep manifest error display
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
