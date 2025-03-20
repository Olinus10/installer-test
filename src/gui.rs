use std::{collections::BTreeMap, path::PathBuf};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use dioxus::prelude::*;
use log::{error, debug};
use modal::ModalContext;
use modal::Modal;
use dioxus::events::SerializedFormData;
use std::collections::HashMap;
use crate::{get_app_data, get_installed_packs, get_launcher, uninstall, InstallerProfile, Launcher, PackName};

use std::sync::atomic::{AtomicUsize, Ordering};
use web_sys::js_sys;
use wasm_bindgen::JsCast;

mod modal;

#[derive(Debug, Clone)]
struct TabInfo {
    color: String,
    title: String, 
    background: String,
    settings_background: String,
    primary_font: String,
    secondary_font: String,
    modpacks: Vec<InstallerProfile>,
}

// Home Page component to display all available modpacks as a grid
#[component]
fn HomePage(
    pages: Signal<BTreeMap<usize, TabInfo>>,
    page: Signal<usize>
) -> Element {
    debug!("HomePage component rendering with {} tabs", pages().len());
    
    rsx! {
        div { class: "home-container",
            h1 { class: "home-title", "Available Modpacks" }
            
            div { class: "home-grid",
                for (index, info) in pages() {
                    for modpack in &info.modpacks {
                        {
                            let modpack_subtitle = modpack.manifest.subtitle.clone();
                            let tab_title = info.title.clone(); 
                            let tab_index = index; 
                            
                            // Check if this modpack is trending
                            let is_trending = modpack.manifest.trend.unwrap_or(false);
                            
                            rsx! {
                                // Create a wrapper div for trending modpacks
                                if is_trending {
                                    div { 
                                        class: "trending-card-wrapper",
                                        onclick: move |_| {
                                            let old_page = page();
                                            debug!("HOME CLICK: Changing page from {} to {} ({}) - HOME_PAGE={}", 
                                                old_page, tab_index, tab_title, HOME_PAGE);
                                            
                                            page.write().clone_from(&tab_index);
                                            
                                            let new_page = page();
                                            debug!("HOME CLICK RESULT: Page is now {}", new_page);
                                        },
                                        
                                        // Add the star outside the card but in the wrapper
                                        div { class: "trending-crown" }
                                        
                                        div { 
                                            class: "home-pack-card trending",
                                            style: "background-image: url('{info.background}'); background-color: {info.color};",
                                            
                                            // Add trending badge
                                            div { class: "trending-badge", "Popular" }
                                            
                                            div { class: "home-pack-info",
                                                h2 { class: "home-pack-title", "{modpack_subtitle}" }
                                                div { class: "home-pack-button", "View Modpack" }
                                            }
                                        }
                                    }
                                } else {
                                    // Regular non-trending card
                                    div { 
                                        class: "home-pack-card",
                                        style: "background-image: url('{info.background}'); background-color: {info.color};",
                                        onclick: move |_| {
                                            let old_page = page();
                                            debug!("HOME CLICK: Changing page from {} to {} ({}) - HOME_PAGE={}", 
                                                old_page, tab_index, tab_title, HOME_PAGE);
                                            
                                            page.write().clone_from(&tab_index);
                                            
                                            let new_page = page();
                                            debug!("HOME CLICK RESULT: Page is now {}", new_page);
                                        },
                                        
                                        div { class: "home-pack-info",
                                            h2 { class: "home-pack-title", "{modpack_subtitle}" }
                                            div { class: "home-pack-button", "View Modpack" }
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
// Special value for home page
const HOME_PAGE: usize = usize::MAX;


#[component]
fn ProgressView(value: i64, max: i64, status: String, title: String) -> Element {
    rsx!(
        div { class: "progress-container",
            div { class: "progress-header",
                h1 { "{title}" }
            }
            div { class: "progress-content",
                progress { class: "progress-bar", max, value: "{value}" }
                p { class: "progress-status", "{status}" }
            }
        }
    )
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

fn feature_change(
    local_features: Signal<Option<Vec<String>>>,
    mut modify: Signal<bool>,
    evt: FormEvent,
    feat: &super::Feature,
    mut modify_count: Signal<i32>,
    mut enabled_features: Signal<Vec<String>>,
    mut refresh_trigger: Signal<i32>,
) {
    // Get the new enabled state from the event
    let enabled = evt.data.value() == "true";
    
    debug!("Feature toggle changed: {} -> {}", feat.id, enabled);
    
    // Update enabled_features collection
    let mut features = enabled_features.read().clone();
    if enabled {
        if !features.contains(&feat.id) {
            features.push(feat.id.clone());
            debug!("Added feature to enabled list: {}", feat.id);
        }
    } else {
        features.retain(|id| id != &feat.id);
        debug!("Removed feature from enabled list: {}", feat.id);
    }
    enabled_features.set(features);
    
    // Check if this is a modification from the original state
    if let Some(local_feat) = local_features.read().as_ref() {
        let was_originally_enabled = local_feat.contains(&feat.id);
        let is_modified = was_originally_enabled != enabled;
        
        if is_modified {
            let new_count = *modify_count.read() + 1;
            modify_count.set(new_count);
            if new_count > 0 {
                modify.set(true);
            }
        } else {
            let new_count = (*modify_count.read() - 1).max(0);
            modify_count.set(new_count);
            if new_count <= 0 {
                modify.set(false);
            }
        }
    }
    
    // Force a UI refresh
    refresh_trigger.with_mut(|x| *x += 1);
}

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
    
    // Use a consistent font for all tabs/components - use the Wynncraft Game Font
    let consistent_font = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2".to_string();
    
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

        // Use a consistent font for all purposes
        let tab_info = TabInfo {
            color: tab_color,
            title: tab_title,
            background: tab_background,
            settings_background,
            primary_font: consistent_font.clone(),
            secondary_font: consistent_font.clone(),
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
    
    // State signals with proper initialization
    let mut installing = use_signal(|| false);
    let mut progress_status = use_signal(|| "".to_string());
    let mut install_progress = use_signal(|| 0);
    let mut modify = use_signal(|| false);
    let mut credits = use_signal(|| false);
    let mut install_item_amount = use_signal(|| 0);
    let mut installed = use_signal(|| installer_profile.installed);
    let mut update_available = use_signal(|| installer_profile.update_available);
    
    // Create a stable feature list signal
    let mut feature_list = use_signal(|| {
        let mut features = vec!["default".to_string()];
        
        if installer_profile.installed && installer_profile.local_manifest.is_some() {
            features = installer_profile.local_manifest.as_ref().unwrap().enabled_features.clone();
        } else {
            // Add default features
            for feat in &installer_profile.manifest.features {
                if feat.default {
                    features.push(feat.id.clone());
                }
            }
        }
        
        debug!("Initialized features: {:?}", features);
        features
    });
    
    // Keep track of original feature state to detect modifications
    let original_features = use_memo(move || {
        if let Some(ref manifest) = installer_profile.local_manifest {
            manifest.enabled_features.clone()
        } else {
            feature_list.read().clone()
        }
    });
    
    // Function to toggle a feature and update modification state
    let toggle_feature = move |feat_id: String| {
        let mut features = feature_list.read().clone();
        let is_enabled = features.contains(&feat_id);
        
        if is_enabled {
            features.retain(|id| id != &feat_id);
            debug!("Removed feature: {}", feat_id);
        } else {
            features.push(feat_id.clone());
            debug!("Added feature: {}", feat_id);
        }
        
        // Update the feature list
        feature_list.set(features.clone());
        
        // Check if this is a modification from original state
        let original_state = original_features.read();
        let was_originally_enabled = original_state.contains(&feat_id);
        let is_modified = features.iter().any(|id| !original_state.contains(id)) || 
                           original_state.iter().any(|id| !features.contains(id));
        
        modify.set(is_modified);
        debug!("Modification state: {}", is_modified);
    };
    
    // Your existing on_submit handler
    let movable_profile = installer_profile.clone();
    let on_submit = move |_| {
        // Calculate total items for progress tracking
        install_item_amount.set(movable_profile.manifest.mods.len()
            + movable_profile.manifest.resourcepacks.len()
            + movable_profile.manifest.shaderpacks.len()
            + movable_profile.manifest.include.len());
        
        let movable_profile = movable_profile.clone();
        let movable_profile2 = movable_profile.clone();
        let features_for_install = feature_list.read().clone();
        
        async move {
            // Make sure the capture of features_for_install is separate from the actual installation closure
            let install_features = features_for_install.clone();
            
            let install = move |canceled| {
                let mut installer_profile = movable_profile.clone();
                let features_clone = install_features.clone();
                
                spawn(async move {
                    if canceled {
                        return;
                    }
                    installing.set(true);
                    
                    // Use our feature list for installation
                    installer_profile.enabled_features = features_clone;
                    installer_profile.manifest.enabled_features = installer_profile.enabled_features.clone();

                    if !*installed.read() {
                        progress_status.set("Installing".to_string());
                        match crate::install(&installer_profile, move || {
                            install_progress.with_mut(|x| *x += 1);
                        })
                        .await
                        {
                            Ok(_) => {
                                installed.set(true);
                                debug!("Installation completed successfully");
                                
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
                        update_available.set(false);
                    }
                    installing.set(false);
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
    
    // Compute button label and state based on signals
    let button_label = if !*installed.read() {
        "Install"
    } else if *update_available.read() {
        "Update"
    } else if *modify.read() {
        "Modify"
    } else {
        "Installed"
    };
    
    let button_disabled = *installed.read() && !*update_available.read() && !*modify.read();
    
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
                enabled: feature_list.read().clone(),
                credits
            }
        } else {
            div { 
                class: "version-container",
                
                form { onsubmit: on_submit,
                    // Header section with title and subtitle
                    div { class: "content-header",
                        h1 { "{installer_profile.manifest.subtitle}" }
                    }
                    
                    // Description section
                    div { class: "content-description",
                        dangerous_inner_html: "{installer_profile.manifest.description}",
                        
                        // Credits link
                        div {
                            a {
                                class: "credits-link",
                                onclick: move |evt| {
                                    credits.set(true);
                                    evt.stop_propagation();
                                },
                                "View Credits"
                            }
                        }
                    }
                    
                    // Features heading
                    h2 { "Optional Features" }
                    
                    // Feature cards - simplified implementation
                    div { class: "feature-cards-container",
                        for feat in installer_profile.manifest.features.iter() {
                            if !feat.hidden {
                                {
                                    // Get current state by checking our feature list
                                    let feat_id = feat.id.clone();
                                    let is_enabled = feature_list.read().contains(&feat_id);
                                    
                                    rsx! {
                                        div { 
                                            class: if is_enabled { "feature-card feature-enabled" } else { "feature-card feature-disabled" },
                                            h3 { class: "feature-card-title", "{feat.name}" }
                                            
                                            // Description if available
                                            if let Some(description) = &feat.description {
                                                div { class: "feature-card-description", "{description}" }
                                            }
                                            
                                            // Toggle button with direct handler
                                            div {
                                                class: if is_enabled { "feature-toggle-button enabled" } else { "feature-toggle-button disabled" },
                                                onclick: move |_| {
                                                    let feat_id = feat_id.clone();
                                                    toggle_feature(feat_id);
                                                },
                                                if is_enabled { "Enabled" } else { "Disabled" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Install/Update/Modify button with simplified state
                    div { class: "install-button-container",
                        button {
                            class: "main-install-button",
                            disabled: button_disabled,
                            "{button_label}"
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
    // Organize tabs into main tabs and dropdown tabs
    let pages_value = pages.read();
    
    let mut main_tabs = Vec::new();
    let mut dropdown_tabs = Vec::new();
    
    for (idx, info) in pages_value.iter() {
        if *idx >= 1 && *idx <= 3 {
            main_tabs.push((*idx, info.title.clone()));
        } else {
            dropdown_tabs.push((*idx, info.title.clone()));
        }
    }
    
    let has_dropdown = !dropdown_tabs.is_empty();
    let current_page = *page.read();
    let any_dropdown_active = dropdown_tabs.iter().any(|(idx, _)| current_page == *idx);
  
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
                    },
                    style: "cursor: pointer;"
                }
            }
            
            h1 { 
                class: "app-title", 
                onclick: move |_| {
                    page.set(HOME_PAGE);
                },
                style: "cursor: pointer;",
                "Overhaul Installer" 
            }
            
            // Tabs from pages - show only if we have pages
            div { class: "header-tabs",
                // Home tab
                button {
                    class: if current_page == HOME_PAGE { "header-tab-button active" } else { "header-tab-button" },
                    onclick: move |_| {
                        page.set(HOME_PAGE);
                    },
                    "Home"
                }

                // Main tabs (1, 2, 3)
                for (idx, title) in main_tabs {
                    button {
                        class: if current_page == idx { "header-tab-button active" } else { "header-tab-button" },
                        onclick: move |_| {
                            page.set(idx);
                        },
                        "{title}"
                    }
                }
                
                // Dropdown for remaining tabs
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
                            for (idx, title) in dropdown_tabs {
                                button {
                                    class: if current_page == idx { "dropdown-item active" } else { "dropdown-item" },
                                    onclick: move |_| {
                                        page.set(idx);
                                    },
                                    "{title}"
                                }
                            }
                        }
                    }
                } else if pages_value.is_empty() {
                    // If no tabs, show a message
                    span { style: "color: #888; font-style: italic;", "Loading tabs..." }
                }
            }
            
            // Settings button
            button {
                class: "settings-button",
                onclick: move |_| {
                    settings.set(true);
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
    let err: Signal<Option<String>> = use_signal(|| None);
    let page = use_signal(|| HOME_PAGE);
    let pages = use_signal(BTreeMap::<usize, TabInfo>::new);

    let cfg = config.with(|cfg| cfg.clone());
    let launcher = match super::get_launcher(&cfg.launcher) {
        Ok(val) => Some(val),
        Err(e) => {
            err.set(Some(format!("Failed to load launcher: {}", e)));
            None
        },
    };

    // Process branches more efficiently
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
            let mut new_pages = BTreeMap::<usize, TabInfo>::new();
            
            for (tab_group, profile) in processed_branches {
                let tab_title = profile.manifest.tab_title.clone()
                    .unwrap_or_else(|| profile.manifest.subtitle.clone());
                    
                let tab_color = profile.manifest.tab_color.clone()
                    .unwrap_or_else(|| String::from("#320625"));
                    
                let tab_background = profile.manifest.tab_background.clone()
                    .unwrap_or_else(|| String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png"));
                    
                let settings_background = profile.manifest.settings_background.clone()
                    .unwrap_or_else(|| tab_background.clone());
                    
                let primary_font = profile.manifest.tab_primary_font.clone()
                    .unwrap_or_else(|| String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2"));
                    
                let secondary_font = profile.manifest.tab_secondary_font.clone()
                    .unwrap_or_else(|| primary_font.clone());

                if let Some(tab_info) = new_pages.get_mut(tab_group) {
                    // Add to existing tab group
                    tab_info.modpacks.push(profile.clone());
                } else {
                    // Create new tab group
                    new_pages.insert(*tab_group, TabInfo {
                        color: tab_color,
                        title: tab_title,
                        background: tab_background,
                        settings_background,
                        primary_font,
                        secondary_font,
                        modpacks: vec![profile.clone()],
                    });
                }
            }
            
            pages.set(new_pages);
        }
    });

    // Generate CSS content
    let css_content = {
        let current_page = *page.read();
        let all_pages = pages.read().clone();
        let is_settings = *settings.read();
        
        let default_color = "#320625".to_string();
        let default_bg = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png".to_string();
        let default_font = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2".to_string();
        
        let bg_color = all_pages.get(&current_page)
            .map(|x| x.color.clone())
            .unwrap_or(default_color);
        
        let bg_image = if is_settings {
            all_pages.get(&current_page)
                .map(|x| x.settings_background.clone())
                .unwrap_or(default_bg)
        } else {
            all_pages.get(&current_page)
                .map(|x| x.background.clone())
                .unwrap_or(default_bg)
        };
        
        let secondary_font = all_pages.get(&current_page)
            .map(|x| x.secondary_font.clone())
            .unwrap_or(default_font.clone());
        
        let primary_font = all_pages.get(&current_page)
            .map(|x| x.primary_font.clone())
            .unwrap_or(default_font);
            
        // Dropdown CSS styles
        let dropdown_css = "
        /* Dropdown styles */
        .dropdown { 
            position: relative; 
            display: inline-block; 
        }

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

        .dropdown:hover .dropdown-content,
        .dropdown-content:hover {
            display: block;
        }

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
            .replace("<SECONDARY_FONT>", &secondary_font)
            .replace("<PRIMARY_FONT>", &primary_font) 
            + dropdown_css
    };

    let modal_context = use_context_provider(ModalContext::default);
    
    if let Some(e) = err.read().clone() {
        modal_context.open("Error", rsx! {
            p {
                "The installer encountered an error. If the problem does not resolve itself please open a thread in #📂modpack-issues on the discord."
            }
            textarea { class: "error-area", readonly: true, "{e}" }
        }, false, Some(move |_| err.set(None)));
    }

    // Use a consistent logo
    let logo_url = Some("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/icon.png".to_string());
    
    // Main app rendering
    let current_page = *page.read();
    let is_settings = *settings.read();
    let pages_value = pages.read().clone();
    
    rsx! {
        div {
            style { {css_content} }
            Modal {}
            
            // Only show header if not in settings and launcher is valid
            if !config.read().first_launch.unwrap_or(true) && launcher.is_some() && !is_settings {
                AppHeader {
                    page,
                    pages,
                    settings,
                    logo_url
                }
            }

            div { class: "main-container",
                if is_settings {
                    Settings {
                        config,
                        settings,
                        config_path: props.config_path.clone(),
                        error: err,
                        b64_id: URL_SAFE_NO_PAD.encode(props.modpack_source.clone())
                    }
                } else if config.read().first_launch.unwrap_or(true) || launcher.is_none() {
                    Launcher {
                        config,
                        config_path: props.config_path.clone(),
                        error: err,
                        b64_id: URL_SAFE_NO_PAD.encode(props.modpack_source.clone())
                    }
                } else if packs.read().is_none() {
                    div { class: "loading-container",
                        div { class: "loading-spinner" }
                        div { class: "loading-text", "Loading modpack information..." }
                    }
                } else if current_page == HOME_PAGE {
                    HomePage {
                        pages,
                        page
                    }
                } else {
                    // Content for specific page
                    if let Some(tab_info) = pages_value.get(&current_page) {
                        let modpacks = tab_info.modpacks.clone();
                        
                        rsx! {
                            div { 
                                class: "version-page-container",
                                
                                for profile in modpacks {
                                    Version {
                                        installer_profile: profile,
                                        error: err,
                                        current_page,
                                        tab_group: current_page
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            div { "No modpack information found for this tab." }
                        }
                    }
                }
            }
        }
    }
}
