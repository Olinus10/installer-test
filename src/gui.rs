use std::{collections::BTreeMap, path::PathBuf};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use dioxus::prelude::*;
use log::{error, debug};
use modal::ModalContext;
use modal::Modal; 
use crate::{get_app_data, get_installed_packs, get_launcher, uninstall, InstallerProfile, Launcher, PackName, Changelog,launcher::launch_modpack};

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
pub fn PlayButton(
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

#[derive(Debug, Clone, PartialEq)]
pub enum AuthStatus {
    Authenticated,  // User already authenticated
    NeedsAuth,      // User needs to authenticate first
}

// Helper function to check auth status of a profile
pub fn get_auth_status() -> AuthStatus {
    if crate::launcher::MicrosoftAuth::is_authenticated() {
        AuthStatus::Authenticated
    } else {
        AuthStatus::NeedsAuth
    }
}

// Enhanced handler for play button clicks
pub fn handle_play_click(uuid: String, error_signal: &Signal<Option<String>>) {
    debug!("Play button clicked for modpack: {}", uuid);
    
    // Launch the modpack
    std::thread::spawn(move || {
        match crate::launcher::launch_modpack(&uuid) {
            Ok(_) => {
                debug!("Successfully launched modpack: {}", uuid);
            },
            Err(e) => {
                error!("Failed to launch modpack: {}", e);
                error_signal.set(Some(format!("Failed to launch modpack: {}", e)));
            }
        }
    });
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
fn HomePage(
    pages: Signal<BTreeMap<usize, TabInfo>>,
    page: Signal<usize>
) -> Element {
    debug!("HomePage component rendering with {} tabs", pages().len());
    
    // Get the changelog from the first modpack profile we can find
    let changelog = pages().values().next().and_then(|tab_info| {
        tab_info.modpacks.first().and_then(|profile| profile.changelog.clone())
    });
    
    // Error signal for modpack launching
    let mut err = use_signal(|| Option::<String>::None);
    
    rsx! {
        if let Some(error) = err() {
            div { class: "error-notification",
                div { class: "error-message", "{error}" }
                button { 
                    class: "error-close",
                    onclick: move |_| err.set(None),
                    "×"
                }
            }
        }
        
        div { class: "home-container",
            // Add Statistics Display
            StatisticsDisplay {}
            
            // Add a section divider with title
            div { class: "section-divider with-title", 
                span { class: "divider-title", "FEATURED MODPACKS" }
            }
            
            div { class: "home-grid",
                for (index, info) in pages() {
                    for modpack in &info.modpacks {
                        {
                            let modpack_subtitle = modpack.manifest.subtitle.clone();
                            let tab_title = info.title.clone(); 
                            let tab_index = index; 
                            
                            // Get metadata for enhanced card presentation
                            let category = modpack.manifest.category.clone().unwrap_or_else(|| "Gameplay".to_string());
                            let is_trending = modpack.manifest.trend.unwrap_or(false);
                            let is_updated = modpack.update_available;
                            let is_new = modpack.manifest.is_new.unwrap_or(false);
                            let description = modpack.manifest.short_description.clone();
                            let is_installed = modpack.installed;
                            let uuid = modpack.manifest.uuid.clone();
                            
                            rsx! {
                                // Create a wrapper div for trending modpacks
                                if is_trending {
                                    div { 
                                        class: "trending-card-wrapper",
                                        
                                        // Add the crown outside the card but in the wrapper
                                        div { class: "trending-crown" }
                                        
                                        div { 
                                            class: "home-pack-card trending",
                                            style: "background-image: url('{info.background}'); background-color: {info.color};",
                                            "data-category": "{category}",
                                            
                                            // Category badge
                                            div { class: "category-badge {category.to_lowercase()}", "{category}" }
                                            
                                            // Add trending badge
                                            div { class: "trending-badge", "Popular" }
                                            
                                            div { class: "home-pack-info",
                                                h2 { class: "home-pack-title", "{modpack_subtitle}" }
                                                
                                                // Description (hidden until hover)
                                                if let Some(desc) = &description {
                                                    div { class: "home-pack-description", "{desc}" }
                                                }
                                                
                                                div { 
                                                    class: "home-pack-button",
                                                    onclick: move |_| {
                                                        let old_page = page();
                                                        debug!("HOME CLICK: Changing page from {} to {} ({}) - HOME_PAGE={}", 
                                                            old_page, tab_index, tab_title, HOME_PAGE);
                                                        
                                                        page.write().clone_from(&tab_index);
                                                        
                                                        let new_page = page();
                                                        debug!("HOME CLICK RESULT: Page is now {}", new_page);
                                                    },
                                                    "View Modpack" 
                                                }
                                                
                                                // Add Play button if installed
                                                if is_installed {
                                                    {
                                                        let uuid_clone = uuid.clone();
                                                        let mut err_clone = err.clone();
                                                        
                                                        rsx! {
                                                            div { 
                                                                class: "home-pack-play-button",
                                                                onclick: move |evt| {
                                                                    evt.stop_propagation(); // Prevent navigation
                                                                    
                                                                    // Launch the modpack
                                                                    debug!("Launching modpack with UUID: {}", uuid_clone);
                                                                    match crate::launcher::launch_modpack(&uuid_clone) {
                                                                        Ok(_) => debug!("Successfully launched modpack: {}", uuid_clone),
                                                                        Err(e) => err_clone.set(Some(format!("Failed to launch modpack: {}", e)))
                                                                    }
                                                                },
                                                                "PLAY"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Regular non-trending card
                                    div { 
                                        class: "home-pack-card",
                                        style: "background-image: url('{info.background}'); background-color: {info.color};",
                                        "data-category": "{category}",
                                        "data-new": "{is_new}",
                                        "data-updated": "{is_updated}",
                                        
                                        // Category badge
                                        div { class: "category-badge {category.to_lowercase()}", "{category}" }
                                        
                                        // NEW/UPDATED ribbon if applicable
                                        if is_new {
                                            div { class: "new-ribbon", "NEW" }
                                        } else if is_updated {
                                            div { class: "updated-ribbon", "UPDATED" }
                                        }
                                        
                                        div { class: "home-pack-info",
                                            h2 { class: "home-pack-title", "{modpack_subtitle}" }
                                            
                                            // Description (hidden until hover)
                                            if let Some(desc) = &description {
                                                div { class: "home-pack-description", "{desc}" }
                                            }
                                            
                                            div { 
                                                class: "home-pack-button",
                                                onclick: move |_| {
                                                    let old_page = page();
                                                    debug!("HOME CLICK: Changing page from {} to {} ({}) - HOME_PAGE={}", 
                                                        old_page, tab_index, tab_title, HOME_PAGE);
                                                    
                                                    page.write().clone_from(&tab_index);
                                                    
                                                    let new_page = page();
                                                    debug!("HOME CLICK RESULT: Page is now {}", new_page);
                                                },
                                                "View Modpack" 
                                            }
                                            
                                            // Add Play button if installed
                                            if is_installed {
                                                {
                                                    let uuid_clone = uuid.clone();
                                                    let mut err_clone = err.clone();
                                                    
                                                    rsx! {
                                                        div { 
                                                            class: "home-pack-play-button",
                                                            onclick: move |evt| {
                                                                evt.stop_propagation(); // Prevent navigation
                                                                
                                                                // Launch the modpack
                                                                debug!("Launching modpack with UUID: {}", uuid_clone);
                                                                match crate::launcher::launch_modpack(&uuid_clone) {
                                                                    Ok(_) => debug!("Successfully launched modpack: {}", uuid_clone),
                                                                    Err(e) => err_clone.set(Some(format!("Failed to launch modpack: {}", e)))
                                                                }
                                                            },
                                                            "PLAY"
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
            
            // Add Changelog Section
            ChangelogSection { changelog: changelog }
            
            // Add a section divider
            div { class: "section-divider animated" }
        }
        
        // Add Footer
        Footer {}
    }
}
// Special value for home page
const HOME_PAGE: usize = usize::MAX;

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

    // Play button handler - use the pre-cloned UUID
    let uuid_clone = uuid.clone();
    let on_play = move |_| {
        debug!("Launching modpack with UUID: {}", uuid_clone);
        match crate::launcher::launch_modpack(&uuid_clone) {
            Ok(_) => {
                debug!("Successfully launched modpack: {}", uuid_clone);
            }
            Err(e) => {
                props.error.set(Some(format!("Failed to launch modpack: {}", e)));
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
                        div { class: "play-button-container", style: "margin-top: 20px; text-align: center;" }
                        PlayButton {
                            uuid: uuid.clone(),
                            disabled: false,
                            onclick: on_play
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
    let dropdown_css = "
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
        HomePage {
            pages,
            page
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
                            let mut error_signal = error_msg.clone();
                            
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
                                            div { class: "play-button-container", style: "margin-top: 15px;" }
                                            button {
                                                class: "main-play-button",
                                                onclick: move |_| {
                                                    let uuid_clone = uuid.clone();
                                                    debug!("Launching modpack with UUID: {}", uuid_clone);
                                                    match crate::launcher::launch_modpack(&uuid_clone) {
                                                        Ok(_) => debug!("Successfully launched modpack: {}", uuid_clone),
                                                        Err(e) => error_signal.set(Some(format!("Failed to launch modpack: {}", e)))
                                                    }
                                                },
                                                "PLAY"
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
}}}}}}
