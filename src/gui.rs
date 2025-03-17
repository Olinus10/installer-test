use std::{collections::BTreeMap, path::PathBuf};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use dioxus::prelude::*;
use log::{error, debug};
use modal::ModalContext;
use modal::Modal;

// Import the necessary types from the main module - note the correct names!
use crate::{
    get_app_data, get_installed_packs, get_launcher, uninstall, 
    InstallerProfile, Launcher, PackName
};

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
                    // Only include tab_group >= 1 (skip 0 as requested)
                    if index >= 1 {
                        for modpack in &info.modpacks {
                            {
                                let modpack_subtitle = modpack.manifest.subtitle.clone();
                                let tab_title = info.title.clone(); 
                                let tab_index = index;
                                let is_trending = modpack.manifest.trend.unwrap_or(false);
                                
                                rsx! {
                                    div { 
                                        class: if is_trending { "home-pack-card trending" } else { "home-pack-card" },
                                        style: "background-image: url('{info.background}'); background-color: {info.color};",
                                        onclick: move |_| {
                                            // Log current page and target
                                            debug!("HOME CLICK: Changing page from {} to {} ({}) - HOME_PAGE={}", 
                                                page(), tab_index, tab_title, HOME_PAGE);
                                            
                                            // Set the page explicitly instead of using clone_from
                                            page.set(tab_index);
                                            
                                            // Double-check update worked
                                            debug!("HOME CLICK RESULT: Page is now {}", page());
                                        },
                                        
                                        // Add the trending marker for popular packs
                                        if is_trending {
                                            div { class: "trending-badge", "Popular" }
                                        }
                                        
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

#[component]
fn FeatureCard(props: FeatureCardProps) -> Element {
    let enabled = props.enabled;
    let feature_id = props.feature.id.clone();
    
    rsx! {
        div { 
            class: if enabled { "feature-card feature-enabled" } else { "feature-card feature-disabled" },
            h3 { class: "feature-card-title", "{props.feature.name}" }
            
            // Render description if available
            if let Some(description) = &props.feature.description {
                div { class: "feature-card-description", "{description}" }
            }
            
            // Toggle button with hidden checkbox for better interactivity
            label {
                class: if enabled { "feature-toggle-button enabled" } else { "feature-toggle-button disabled" },
                input {
                    r#type: "checkbox",
                    name: "{feature_id}",
                    checked: if enabled { Some("true") } else { None },
                    onchange: move |evt| props.on_toggle.call(evt),
                    style: "display: none;"
                }
                if enabled { "Enabled" } else { "Disabled" }
            }
        }
    }
}

// Fix for feature_change function to properly handle toggling
fn feature_change(
    local_features: Signal<Option<Vec<String>>>,
    mut modify: Signal<bool>,
    evt: FormEvent,
    feat: &crate::Feature,  // Use full path here
    mut modify_count: Signal<i32>,
    mut enabled_features: Signal<Vec<String>>,
) {
    // Extract values first
    let enabled = match &*evt.data.value() {
        "true" => true,
        "false" => false,
        _ => panic!("Invalid bool from feature"),
    };
    
    // Copy values we need for comparison
    let current_features = enabled_features.read().clone();
    let contains_feature = current_features.contains(&feat.id);
    let current_count = *modify_count.read();
    
    // Only update if necessary
    if enabled != contains_feature {
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

async fn init_branch(source: String, branch: String, launcher: Launcher, mut pages: Signal<BTreeMap<usize, TabInfo>>) -> Result<(), String> {
    debug!("Initializing modpack from source: {}, branch: {}", source, branch);
    let profile = crate::init(source.to_owned(), branch.to_owned(), launcher).await?;

    // Process manifest data for tab information
    debug!("Processing manifest tab information:");
    debug!("  subtitle: {}", profile.manifest.subtitle);
    debug!("  description length: {}", profile.manifest.description.len());

    // Adjust tab_group to start from 1 instead of 0
    let tab_group = if let Some(tab_group) = profile.manifest.tab_group {
        debug!("  Original tab_group: {}", tab_group);
        // Ensure it's at least 1
        if tab_group == 0 { 1 } else { tab_group }
    } else {
        debug!("  tab_group: None, defaulting to 1");
        1 // Default to tab group 1 instead of 0
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
    
    debug!("Version component for '{}' - current_page: {}, tab_group: {}",
           installer_profile.manifest.subtitle,
           props.current_page,
           props.tab_group);

    let mut installing = use_signal(|| false);
    let mut progress_status = use_signal(|| "");
    let mut install_progress = use_signal(|| 0);
    let mut modify = use_signal(|| false);
    let mut modify_count = use_signal(|| 0);

    // Fix: Initialize enabled_features properly
    let enabled_features = use_signal(|| {
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

        debug!("Initial enabled features for '{}': {:?}",
               installer_profile.manifest.subtitle, features);
        features
    });

    let mut install_item_amount = use_signal(|| 0);
    let mut credits = use_signal(|| false);
    let installed = use_signal(|| installer_profile.installed);
    let mut update_available = use_signal(|| installer_profile.update_available);
    
    // Clone local_manifest to prevent ownership issues
    let mut local_features = use_signal(|| {
        if let Some(ref manifest) = installer_profile.local_manifest {
            Some(manifest.enabled_features.clone())
        } else {
            None
        }
    });
    
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
                        progress_status.set("Installing");
                        match crate::install(&installer_profile, move || {
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
                        progress_status.set("Modifying");
                        match super::update(&installer_profile, move || {
                            *install_progress.write() += 1
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
                        modify.with_mut(|x| *x = false);
                        modify_count.with_mut(|x| *x = 0);
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

    let install_disable = if *installed.read() && !*update_available.read() && !*modify.read() {
        Some("true")
    } else {
        None
    };
    
    // Log the features to help debug
    debug!("Modpack '{}' has {} features", 
           installer_profile.manifest.subtitle, 
           installer_profile.manifest.features.len());
           
    for feat in &installer_profile.manifest.features {
        debug!("Feature: id={}, name={}, hidden={}", feat.id, feat.name, feat.hidden);
    }
    
    rsx! {
        if *installing.read() {
            ProgressView {
                value: install_progress(),
                max: install_item_amount() as i64,
                title: installer_profile.manifest.subtitle,
                status: progress_status.to_string()
            }
        } else if *credits.read() {
            Credits {
                manifest: installer_profile.manifest,
                enabled: installer_profile.enabled_features,
                credits
            }
        } else {
            div { class: "version-container",
                form { onsubmit: on_submit,
                    // Improved streamlined header section with title and subtitle
                    div { class: "content-header",
                        h1 { "{installer_profile.manifest.subtitle}" }
                    }
                    
                    // Clean description section with no box
                    div { class: "content-description-clean",
                        // The 'dangerous_inner_html' directive renders HTML content safely
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
                    
                    // Compact feature cards in a responsive grid
                    div { class: "feature-cards-container",
                        for feat in installer_profile.manifest.features {
                            if !feat.hidden {
                                {
                                    let feat_id = feat.id.clone();
                                    let feat_clone = feat.clone(); // Clone feature for use in closure
                                    let is_enabled = enabled_features.read().contains(&feat_id) || feat.default;
                                    
                                    rsx! {
                                        div { 
                                            class: if is_enabled { "feature-card feature-enabled" } else { "feature-card feature-disabled" },
                                            h3 { class: "feature-card-title", "{feat.name}" }
                                            
                                            // Render description if available
                                            if let Some(description) = &feat.description {
                                                div { class: "feature-card-description", "{description}" }
                                            }
                                            
                                            // Improved toggle button with clickable functionality
                                            label {
                                                class: if is_enabled { "feature-toggle-button enabled" } else { "feature-toggle-button disabled" },
                                                
                                                // Hidden checkbox to track state
                                                input {
                                                    r#type: "checkbox",
                                                    name: "{feat_id}",
                                                    checked: if is_enabled { Some("true") } else { None },
                                                    onchange: move |evt| {
                                                        // Call feature_change to toggle the feature
                                                        feature_change(
                                                            local_features,
                                                            modify,
                                                            evt,
                                                            &feat_clone,
                                                            modify_count,
                                                            enabled_features,
                                                        );
                                                    },
                                                    style: "display: none;"
                                                }
                                                
                                                if is_enabled { "Enabled" } else { "Disabled" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Prominent and centered install button
                    div { class: "install-button-container-centered",
                        button {
                            class: "main-install-button",
                            disabled: install_disable,
                            if !*installed.read() {
                                "Install"
                            } else if *update_available.read() {
                                "Update"
                            } else {
                                "Modify"
                            }
                        }
                    }
                }
            }
        }
    }
}
// New header component with tabs
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
    
    // Separate tab groups into main tabs (1-3) and dropdown tabs (4+)
    for (index, info) in pages().iter() {
        if *index >= 1 && *index <= 3 {
            main_tab_indices.push(*index);
            main_tab_titles.push(info.title.clone());
        } else if *index > 3 {
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
                "Modpack Installer" 
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

                // Main tabs (1-3)
                {
                    main_tab_indices.iter().enumerate().map(|(i, &index)| {
                        let title = main_tab_titles[i].clone();
                        rsx!(
                            button {
                                class: if page() == index { "header-tab-button active" } else { "header-tab-button" },
                                onclick: move |_| {
                                    debug!("TAB CLICK: Changing page from {} to {}", page(), index);
                                    // Use set instead of write().clone_from()
                                    page.set(index);
                                    debug!("TAB CLICK RESULT: Page is now {}", page());
                                },
                                "{title}"
                            }
                        )
                    })
                }
                
                // Dropdown for remaining tabs (4+)
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
                                                // Use set instead of write
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
    // Include both the original CSS and our improved styles
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
                                // Adjust tab_group numbering to start from 1 instead of 0
                                let tab_group = profile.manifest.tab_group.unwrap_or(1);
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
            // Ensure tab_group is at least 1
            // Dereference tab_group when comparing, and use the correct type for the result
            let adjusted_tab_group = if *tab_group == 0 { 1 } else { *tab_group };
            
            // Add debug to see which tab groups are being processed
            debug!("Processing tab_group: {} for profile: {}", 
                   adjusted_tab_group, profile.manifest.subtitle);
            
            let tab_title = profile.manifest.tab_title.clone().unwrap_or_else(|| profile.manifest.subtitle.clone());
            let tab_color = profile.manifest.tab_color.clone().unwrap_or_else(|| String::from("#320625"));
            let tab_background = profile.manifest.tab_background.clone().unwrap_or_else(|| {
                String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png")
            });
            let settings_background = profile.manifest.settings_background.clone().unwrap_or_else(|| tab_background.clone());
            let primary_font = profile.manifest.tab_primary_font.clone().unwrap_or_else(|| {
                String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2")
            });
            let secondary_font = profile.manifest.tab_secondary_font.clone().unwrap_or_else(|| primary_font.clone());

            // Use the adjusted value
            new_pages.entry(adjusted_tab_group).or_insert(TabInfo {
                color: tab_color,
                title: tab_title,
                background: tab_background,
                settings_background,
                primary_font,
                secondary_font,
                modpacks: vec![profile.clone()],
            });
        }
        
        pages.set(new_pages);
        debug!("Updated pages map with {} tabs", pages().len());
    }
});

    // Update the CSS generation section
    let css_content = {
        let default_color = "#320625".to_string();
        let default_bg = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png".to_string();
        let default_font = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2".to_string();
        
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
        
        let secondary_font = match pages().get(&page()) {
            Some(x) => x.secondary_font.clone(),
            None => default_font.clone(),
        };
        
        let primary_font = match pages().get(&page()) {
            Some(x) => x.primary_font.clone(),
            None => default_font,
        };
        
        debug!("Updating CSS with: color={}, bg_image={}, secondary_font={}, primary_font={}", 
               bg_color, bg_image, secondary_font, primary_font);
            
        // Our improved dropdown menu CSS with better hover behavior and font consistency
        let improved_css = "
        /* Content header improvements */
        .content-header {
            text-align: center;
            padding: 30px 0 15px 0;
            position: relative;
        }
        
        .content-header h1 {
            font-family: \"SECONDARY_FONT\";
            font-size: 3.5rem;
            margin: 0;
            text-shadow: 0 4px 12px rgba(0, 0, 0, 0.8);
            background: linear-gradient(180deg, #fce8f6 0%, #d8b8d0 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            animation: glow 2s ease-in-out infinite alternate;
            position: relative;
            display: inline-block;
        }
        
        /* Clean description without the box */
        .content-description-clean {
            max-width: 85%;
            margin: 0 auto 30px auto;
            line-height: 1.6;
            text-align: center;
            position: relative;
            font-size: 1.1rem;
            text-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
        }
        
        .content-description-clean p:first-child {
            font-size: 1.2rem;
            margin-top: 0;
        }
        
        /* Feature cards container - more compact */
        .feature-cards-container {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
            gap: 15px;
            margin-bottom: 40px;
            padding: 5px;
            max-height: none;
            overflow-y: visible;
        }
        
        /* Feature cards - smaller, more compact */
        .feature-card {
            background-color: rgba(0, 0, 0, 0.7);
            border-radius: 8px;
            padding: 12px;
            transition: transform 0.2s ease, box-shadow 0.2s ease;
            display: flex;
            flex-direction: column;
            height: auto;
            min-height: 120px;
            max-height: 220px;
            overflow-y: auto;
        }
        
        .feature-card.feature-enabled {
            border-left: 4px solid #073c17;
        }
        
        .feature-card.feature-disabled {
            border-left: 4px solid #d95248;
        }
        
        .feature-card:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 8px rgba(0, 0, 0, 0.3);
        }
        
        .feature-card-title {
            font-family: \"SECONDARY_FONT\";
            margin: 0 0 8px 0;
            font-size: 1.2rem;
        }
        
        .feature-card-description {
            flex-grow: 1;
            margin-bottom: 12px;
            font-size: 0.85rem;
            line-height: 1.4;
            color: #ddd;
            max-height: 80px;
            overflow-y: auto;
        }
        
        /* Feature toggle button improvements */
        .feature-toggle-button {
            align-self: flex-end;
            padding: 6px 14px;
            border-radius: 4px;
            font-family: \"SECONDARY_FONT\";
            text-align: center;
            cursor: pointer;
            transition: all 0.2s ease;
            user-select: none;
            display: inline-block;
            font-size: 0.9rem;
        }
        
        .feature-toggle-button.enabled {
            background-color: #073c17;
        }
        
        .feature-toggle-button.disabled {
            background-color: #d95248;
        }
        
        .feature-toggle-button:hover {
            opacity: 0.9;
            transform: scale(1.05);
            box-shadow: 0 2px 5px rgba(0, 0, 0, 0.4);
        }
        
        /* Install button container - centered and fixed */
        .install-button-container-centered {
            text-align: center;
            margin: 40px 0;
            padding: 10px;
            position: relative;
            width: 100%;
        }
        
        /* Enhanced install button */
        .main-install-button {
            background-color: #073c17;
            border: none;
            border-radius: 8px;
            padding: 18px 70px;
            font-family: \"SECONDARY_FONT\";
            font-size: 2rem;
            box-shadow: 0 6px 12px rgba(0, 0, 0, 0.4), 0 0 20px rgba(7, 60, 23, 0.3);
            transition: all 0.3s ease;
            color: #fce8f6;
            position: relative;
            overflow: hidden;
            cursor: pointer;
            min-width: 320px;
            text-transform: uppercase;
            letter-spacing: 2px;
            /* Ensure the button is fully visible */
            margin-bottom: 60px;
        }
        
        .main-install-button:hover:not([disabled]) {
            transform: translateY(-5px) scale(1.05);
            box-shadow: 0 10px 20px rgba(0, 0, 0, 0.5), 0 0 30px rgba(7, 60, 23, 0.7);
            background-color: #0a4d1e;
            text-shadow: 0 0 10px rgba(255, 255, 255, 0.5);
        }
        
        .main-install-button:active:not([disabled]) {
            transform: translateY(-2px) scale(1.02);
        }
        
        /* Shine effect */
        .main-install-button::before {
            content: '';
            position: absolute;
            top: 0;
            left: -100%;
            width: 100%;
            height: 100%;
            background: linear-gradient(
                90deg, 
                transparent, 
                rgba(255, 255, 255, 0.2), 
                rgba(255, 255, 255, 0.4), 
                transparent
            );
            transition: none;
            z-index: 1;
        }
        
        .main-install-button:hover::before {
            left: 100%;
            transition: left 0.7s ease-in-out;
        }
        
        /* Trending/Popular pack indicator for homepage */
        .home-pack-card.trending {
            box-shadow: 0 6px 12px rgba(0, 0, 0, 0.6), 0 0 0 3px rgba(255, 215, 0, 0.5);
            position: relative;
        }
        
        .trending-badge {
            position: absolute;
            top: 10px;
            right: 10px;
            background-color: rgba(255, 215, 0, 0.9);
            color: #000;
            padding: 4px 12px;
            border-radius: 20px;
            font-size: 0.8rem;
            font-weight: bold;
            box-shadow: 0 2px 5px rgba(0, 0, 0, 0.3);
            text-transform: uppercase;
            z-index: 10;
        }
        
        .home-pack-card.trending:hover {
            box-shadow: 0 10px 25px rgba(0, 0, 0, 0.7), 0 0 0 4px rgba(255, 215, 0, 0.7), 0 0 20px rgba(255, 215, 0, 0.5);
        }
        
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
            /* Explicitly use the same font as header-tab-button */
            font-family: \"PRIMARY_FONT\";
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
            .replace("<SECONDARY_FONT>", &secondary_font)
            .replace("<PRIMARY_FONT>", &primary_font) 
            + improved_css
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
    
    // Current page for rendering decision
    let current_page = page();
    debug!("RENDER DECISION: current_page={}, HOME_PAGE={}, is_home={}",
           current_page, HOME_PAGE, current_page == HOME_PAGE);
    
    rsx! {
        div {
            style { {css_content} }
            Modal {}
            
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
                    let content = {
    let current_page = page();
    debug!("RENDER DECISION: current_page={}, HOME_PAGE={}, is_home={}",
           current_page, HOME_PAGE, current_page == HOME_PAGE);
    
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
            
            // Clone modpacks for rendering
            let modpacks = tab_info.modpacks.clone();
            debug!("Cloned {} modpacks for rendering", modpacks.len());
            
            if !modpacks.is_empty() {
                // Simplified: Just render the first modpack in the Version component
                rsx! {
                    Version {
                        installer_profile: modpacks[0].clone(),
                        error: err.clone(),
                        current_page,
                        tab_group: current_page
                    }
                }
            } else {
                rsx! { div { "No modpack information found for this tab." } }
            }
        } else {
            debug!("NO TAB INFO found for page {}", current_page);
            rsx! { div { "No modpack information found for this tab." } }
        }
    }
};

// Then use content in the appropriate place in your main container
rsx! {
    div { class: "main-container",
        {if settings() {
            // Settings render...
        } else if config.read().first_launch.unwrap_or(true) || launcher.is_none() {
            // Launcher render...
        } else if packs.read().is_none() {
            // Loading render...
        } else {
            // Use the content variable here
            content
        }}
    }
}
                }
            }
        }
    }
}}
