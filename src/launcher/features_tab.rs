use dioxus::prelude::*;
use crate::universal::{ModComponent, UniversalManifest};
use crate::preset::{Preset, find_preset_by_id};
use log::{debug, error};

#[component]
pub fn FeaturesTab(
    universal_manifest: Option<UniversalManifest>,
    presets: Vec<Preset>,
    enabled_features: Signal<Vec<String>>,
    selected_preset: Signal<Option<String>>,
    filter_text: Signal<String>,
    installation_id: String,
) -> Element {
    // Clone for closures
    let presets_for_closure = presets.clone();
    let presets_for_toggle = presets.clone();
    let presets_for_custom_check = presets.clone();
    let installation_id_for_apply = installation_id.clone();
    let installation_id_for_toggle = installation_id.clone();
    let universal_manifest_for_apply = universal_manifest.clone();
    let universal_manifest_for_toggle = universal_manifest.clone();
    let universal_manifest_for_count = universal_manifest.clone();
    let universal_manifest_for_render = universal_manifest.clone();
    
    // FIXED: Enhanced initialization with proper state restoration
    use_effect({
        let installation_id = installation_id.clone();
        let mut selected_preset = selected_preset.clone();
        let mut enabled_features = enabled_features.clone();
        let presets_for_effect = presets.clone();
        
        move || {
            debug!("Initializing features tab for installation: {}", installation_id);
            
            if let Ok(installation) = crate::installation::load_installation(&installation_id) {
                debug!("Loaded installation state:");
                debug!("  Enabled features: {:?}", installation.enabled_features);
                debug!("  Selected preset: {:?}", installation.selected_preset_id);
                debug!("  Base preset: {:?}", installation.base_preset_id);
                debug!("  User optional features: {:?}", installation.user_enabled_optional_features);
                debug!("  User disabled defaults: {:?}", installation.user_disabled_default_features);
                
                // CRITICAL: Set enabled features to what's actually saved
                enabled_features.set(installation.enabled_features.clone());
                
                // CRITICAL: Determine the correct preset selection state
                let effective_preset_id = if installation.user_enabled_optional_features.is_empty() 
                    && installation.user_disabled_default_features.is_empty() {
                    // No customizations - use selected preset
                    installation.selected_preset_id.clone()
                } else {
                    // User has made customizations - check if it matches any preset exactly
                    installation.get_effective_preset_id(&presets_for_effect)
                };
                
                selected_preset.set(effective_preset_id.clone());
                
                debug!("Initialized features tab with preset: {:?}, features: {:?}", 
                       effective_preset_id, installation.enabled_features);
            } else {
                error!("Failed to load installation: {}", installation_id);
            }
        }
    });
    
    // FIXED: Enhanced preset application with proper tracking
    let mut apply_preset = move |preset_id: String| {
        debug!("Applying preset: {}", preset_id);
        
        if preset_id == "custom" {
            // FIXED: Custom preset handling with universal manifest defaults
            let mut default_features = vec!["default".to_string()];
            
            if let Some(manifest) = &universal_manifest_for_apply {
                // Add ALL default-enabled components from universal manifest
                for component in &manifest.mods {
                    if (component.default_enabled || !component.optional) && component.id != "default" 
                       && !default_features.contains(&component.id) {
                        default_features.push(component.id.clone());
                        debug!("Added default mod to custom: {}", component.id);
                    }
                }
                for component in &manifest.shaderpacks {
                    if (component.default_enabled || !component.optional) && component.id != "default" 
                       && !default_features.contains(&component.id) {
                        default_features.push(component.id.clone());
                        debug!("Added default shaderpack to custom: {}", component.id);
                    }
                }
                for component in &manifest.resourcepacks {
                    if (component.default_enabled || !component.optional) && component.id != "default" 
                       && !default_features.contains(&component.id) {
                        default_features.push(component.id.clone());
                        debug!("Added default resourcepack to custom: {}", component.id);
                    }
                }
                
                // FIXED: Handle includes properly
                for include in &manifest.include {
                    if (include.default_enabled || !include.optional) && !include.id.is_empty() 
                       && include.id != "default" && !default_features.contains(&include.id) {
                        default_features.push(include.id.clone());
                        debug!("Added default include to custom: {}", include.id);
                    }
                }
                
                // FIXED: Handle remote includes properly
                for remote in &manifest.remote_include {
                    if (remote.default_enabled || !remote.optional) && remote.id != "default" 
                       && !default_features.contains(&remote.id) {
                        default_features.push(remote.id.clone());
                        debug!("Added default remote include to custom: {}", remote.id);
                    }
                }
            }
            
            debug!("Custom preset will have {} default features: {:?}", default_features.len(), default_features);
            
            enabled_features.set(default_features.clone());
            selected_preset.set(None); // Custom = no preset
            
            // FIXED: Update installation with proper tracking
            if let Ok(mut installation) = crate::installation::load_installation(&installation_id_for_apply) {
                installation.enabled_features = default_features;
                installation.selected_preset_id = None;
                installation.user_enabled_optional_features.clear();
                installation.user_disabled_default_features.clear();
                installation.modified = true;
                
                if let Err(e) = installation.save() {
                    error!("Failed to save installation: {}", e);
                } else {
                    debug!("Saved custom installation state");
                }
            }
        } else if let Some(preset) = find_preset_by_id(&presets_for_closure, &preset_id) {
            debug!("Applying preset '{}' with {} features", preset.name, preset.enabled_features.len());
            
            // Apply preset features
            enabled_features.set(preset.enabled_features.clone());
            selected_preset.set(Some(preset_id.clone()));
            
            // FIXED: Update installation with comprehensive tracking
            if let Ok(mut installation) = crate::installation::load_installation(&installation_id_for_apply) {
                installation.apply_preset_with_tracking(&preset);
                installation.enabled_features = preset.enabled_features.clone();
                installation.modified = true;
                
                if let Err(e) = installation.save() {
                    error!("Failed to save installation: {}", e);
                } else {
                    debug!("Saved preset installation state");
                }
            }
        }
    };
    
    // FIXED: Enhanced feature toggling with dependency handling and proper tracking
    let toggle_feature = move |feature_id: String| {
        debug!("Toggling feature: {}", feature_id);
        let manifest_for_deps = universal_manifest_for_toggle.clone();
        
        enabled_features.with_mut(|features| {
            let is_enabling = !features.contains(&feature_id);
            debug!("Feature '{}' is being {}", feature_id, if is_enabling { "enabled" } else { "disabled" });
            
            if is_enabling {
                if !features.contains(&feature_id) {
                    features.push(feature_id.clone());
                }
                
                // FIXED: Check for dependencies and enable them
                if let Some(manifest) = &manifest_for_deps {
                    let all_components: Vec<&ModComponent> = manifest.mods.iter()
                        .chain(manifest.shaderpacks.iter())
                        .chain(manifest.resourcepacks.iter())
                        .collect();
                    
                    if let Some(component) = all_components.iter().find(|c| c.id == feature_id) {
                        if let Some(deps) = &component.dependencies {
                            for dep_id in deps {
                                if !features.contains(dep_id) {
                                    debug!("Auto-enabling dependency: {} for {}", dep_id, feature_id);
                                    features.push(dep_id.clone());
                                }
                            }
                        }
                    }
                    
                    // FIXED: Also check includes and remote includes for dependencies
                    for include in &manifest.include {
                        if include.id == feature_id {
                            // Handle include dependencies if they exist in the future
                            break;
                        }
                    }
                    
                    for remote in &manifest.remote_include {
                        if remote.id == feature_id {
                            if let Some(deps) = &remote.dependencies {
                                for dep_id in deps {
                                    if !features.contains(dep_id) {
                                        debug!("Auto-enabling dependency: {} for remote include {}", dep_id, feature_id);
                                        features.push(dep_id.clone());
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            } else {
                features.retain(|id| id != &feature_id);
                
                // FIXED: Check if any enabled features depend on this one
                if let Some(manifest) = &manifest_for_deps {
                    let all_components: Vec<&ModComponent> = manifest.mods.iter()
                        .chain(manifest.shaderpacks.iter())
                        .chain(manifest.resourcepacks.iter())
                        .collect();
                    
                    let dependent_features: Vec<String> = all_components.iter()
                        .filter(|c| {
                            features.contains(&c.id) && 
                            c.dependencies.as_ref().map_or(false, |deps| deps.contains(&feature_id))
                        })
                        .map(|c| c.id.clone())
                        .collect();
                    
                    for dep_feat in dependent_features {
                        debug!("Auto-disabling dependent feature: {} (depends on {})", dep_feat, feature_id);
                        features.retain(|id| id != &dep_feat);
                    }
                    
                    // FIXED: Also check remote includes for dependents
                    let dependent_remote_features: Vec<String> = manifest.remote_include.iter()
                        .filter(|r| {
                            features.contains(&r.id) && 
                            r.dependencies.as_ref().map_or(false, |deps| deps.contains(&feature_id))
                        })
                        .map(|r| r.id.clone())
                        .collect();
                    
                    for dep_feat in dependent_remote_features {
                        debug!("Auto-disabling dependent remote feature: {} (depends on {})", dep_feat, feature_id);
                        features.retain(|id| id != &dep_feat);
                    }
                }
            }
        });

        // FIXED: Update installation with enhanced tracking
        if let Ok(mut installation) = crate::installation::load_installation(&installation_id_for_toggle) {
            let http_client = crate::CachedHttpClient::new();
            spawn(async move {
                let is_enabled = enabled_features.read().contains(&feature_id);
                if let Err(e) = installation.toggle_feature_with_tracking(&feature_id, is_enabled, &http_client).await {
                    error!("Failed to update installation feature tracking: {}", e);
                } else {
                    installation.enabled_features = enabled_features.read().clone();
                    installation.modified = true;
                    if let Err(e) = installation.save() {
                        error!("Failed to save installation: {}", e);
                    } else {
                        debug!("Successfully saved feature toggle for: {}", feature_id);
                    }
                }
            });
        }
    };
    
    // Button hover states
    let mut custom_button_hover = use_signal(|| false);
    let mut trending_button_hover = use_signal(Vec::<String>::new);
    let mut regular_button_hover = use_signal(Vec::<String>::new);
    
    // Features section expanded state
    let mut features_expanded = use_signal(|| false);
    
    // Find custom preset for the "Custom Configuration" card
    let custom_preset = presets_for_custom_check.iter().find(|p| p.id == "custom");
    
    rsx! {
        div { class: "features-tab",
            // PRESETS section header
            div { class: "section-divider with-title", 
                span { class: "divider-title", "PRESETS" }
            }
            
            p { class: "section-description", 
                "Choose a preset configuration or customize individual features below."
            }
            
            // Presets grid
            div { class: "presets-grid",
                // Custom preset (no preset selected)
                div { 
                    class: if selected_preset.read().is_none() {
                        "preset-card selected"
                    } else {
                        "preset-card"
                    },
                    style: {
                        if let Some(custom_preset) = custom_preset {
                            if let Some(bg) = &custom_preset.background {
                                format!("background-image: url('{}'); background-size: cover; background-position: center;", bg)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    },
                    onclick: move |_| {
                        apply_preset("custom".to_string());
                    },
                                    
                    div { class: "preset-card-overlay" }
                    
                    div { class: "preset-card-content",
                        h4 { "CUSTOM OVERHAUL" }
                        p { "Start with default features and customize everything yourself." }
                    }
                    
                    // Select/Selected button
                    button {
                        class: "select-preset-button",
                        style: {
                            let is_selected = selected_preset.read().is_none();
                            let is_hovered = *custom_button_hover.read();
                            
                            if is_selected {
                                "background-color: white !important; color: #0a3d16 !important; border: none !important;"
                            } else if is_hovered {
                                "background-color: rgba(10, 80, 30, 0.9) !important; color: white !important; transform: translateX(-50%) translateY(-3px) !important; box-shadow: 0 5px 15px rgba(0, 0, 0, 0.4) !important;"
                            } else {
                                "background-color: rgba(7, 60, 23, 0.7) !important; color: white !important;"
                            }
                        },
                        onmouseenter: move |_| custom_button_hover.set(true),
                        onmouseleave: move |_| custom_button_hover.set(false),
                        
                        if selected_preset.read().is_none() {
                            "SELECTED"
                        } else {
                            "SELECT"
                        }
                    }
                }
                
                // Available presets - skip the "custom" preset since we handle it separately
                for preset in presets.iter().filter(|p| p.id != "custom") {
                    {
                        let preset_id = preset.id.clone();
                        let is_selected = selected_preset.read().as_ref().map_or(false, |id| id == &preset_id);
                        let mut apply_preset_clone = apply_preset.clone();
                        let has_trending = preset.trending.unwrap_or(false);
                        
                        // Check if preset is updated
                        let is_updated = {
                            if let Ok(current_installation) = crate::installation::load_installation(&installation_id) {
                                if let Some(base_preset_id) = &current_installation.base_preset_id {
                                    if base_preset_id == &preset.id {
                                        if let (Some(preset_ver), Some(inst_ver)) = 
                                            (&preset.preset_version, &current_installation.base_preset_version) {
                                            preset_ver != inst_ver
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        };
                        
                        let button_id = preset_id.clone();
                        let is_button_hovered = if has_trending {
                            trending_button_hover.read().contains(&button_id)
                        } else {
                            regular_button_hover.read().contains(&button_id)
                        };
                        
                        rsx! {
                            div {
                                class: if is_selected {
                                    "preset-card selected"
                                } else {
                                    "preset-card"
                                },
                                style: if let Some(bg) = &preset.background {
                                    format!("background-image: url('{}'); background-size: cover; background-position: center;", bg)
                                } else {
                                    String::new()
                                },
                                onclick: move |_| {
                                    apply_preset_clone(preset_id.clone());
                                },
                                
                                span { class: "preset-features-count",
                                    "{preset.enabled_features.len()} features"
                                }
                                
                                if has_trending {
                                    span { class: "trending-badge", "Popular" }
                                }

                                if is_updated {
                                    span { class: "update-badge", "Updated" }
                                }
                                
                                div { class: "preset-card-overlay" }
                                
                                div { class: "preset-card-content",
                                    h4 { "{preset.name}" }
                                    p { "{preset.description}" }
                                }
                                
                                button {
                                    class: "select-preset-button",
                                    style: {
                                        if is_selected {
                                            if has_trending {
                                                "background-color: white !important; color: #b58c14 !important; border: none !important; box-shadow: 0 0 15px rgba(255, 179, 0, 0.3) !important;"
                                            } else {
                                                "background-color: white !important; color: #0a3d16 !important; border: none !important;"
                                            }
                                        } else if is_button_hovered {
                                            if has_trending {
                                                "background: linear-gradient(135deg, #e6b017, #cc9500) !important; color: black !important; transform: translateX(-50%) translateY(-3px) !important; box-shadow: 0 5px 15px rgba(0, 0, 0, 0.4) !important;"
                                            } else {
                                                "background-color: rgba(10, 80, 30, 0.9) !important; color: white !important; transform: translateX(-50%) translateY(-3px) !important; box-shadow: 0 5px 15px rgba(0, 0, 0, 0.4) !important;"
                                            }
                                        } else {
                                            if has_trending {
                                                "background: linear-gradient(135deg, #d4a017, #b78500) !important; color: black !important;"
                                            } else {
                                                "background-color: rgba(7, 60, 23, 0.7) !important; color: white !important;"
                                            }
                                        }
                                    },
                                    onmouseenter: {
                                        let button_id_enter = button_id.clone();
                                        let has_trending_enter = has_trending;
                                        move |_| {
                                            if has_trending_enter {
                                                trending_button_hover.with_mut(|ids| ids.push(button_id_enter.clone()));
                                            } else {
                                                regular_button_hover.with_mut(|ids| ids.push(button_id_enter.clone()));
                                            }
                                        }
                                    },
                                    onmouseleave: {
                                        let button_id_leave = button_id.clone();
                                        let has_trending_leave = has_trending;
                                        move |_| {
                                            if has_trending_leave {
                                                trending_button_hover.with_mut(|ids| ids.retain(|id| id != &button_id_leave));
                                            } else {
                                                regular_button_hover.with_mut(|ids| ids.retain(|id| id != &button_id_leave));
                                            }
                                        }
                                    },
                                    
                                    if is_selected {
                                        "SELECTED"
                                    } else {
                                        "SELECT"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // FEATURES section
            div { class: "optional-features-wrapper",
                div { class: "section-divider with-title", 
                    span { class: "divider-title", "FEATURES" }
                }
                
                p { class: "section-description", 
                    "Customize individual features to create your perfect experience."
                }
                
                // Features count badge
                div { class: "features-count-container",
                    span { class: "features-count-badge",
                        {
                            if let Some(manifest) = &universal_manifest_for_count {
                                let mut total_components = 0;
                                let mut enabled_components = 0;
                                
                                // FIXED: Count all components including includes and remote includes
                                for mod_component in &manifest.mods {
                                    total_components += 1;
                                    if enabled_features.read().contains(&mod_component.id) || 
                                       (mod_component.id == "default" && !mod_component.optional) {
                                        enabled_components += 1;
                                    }
                                }
                                
                                for shader in &manifest.shaderpacks {
                                    total_components += 1;
                                    if enabled_features.read().contains(&shader.id) || 
                                       (shader.id == "default" && !shader.optional) {
                                        enabled_components += 1;
                                    }
                                }
                                
                                for resource in &manifest.resourcepacks {
                                    total_components += 1;
                                    if enabled_features.read().contains(&resource.id) || 
                                       (resource.id == "default" && !resource.optional) {
                                        enabled_components += 1;
                                    }
                                }
                                
                                // FIXED: Count includes properly
                                for include in &manifest.include {
                                    total_components += 1;
                                    let should_include = if include.id.is_empty() || include.id == "default" {
                                        !include.optional
                                    } else if !include.optional {
                                        true
                                    } else {
                                        enabled_features.read().contains(&include.id)
                                    };
                                    
                                    if should_include {
                                        enabled_components += 1;
                                    }
                                }
                                
                                // FIXED: Count remote includes properly
                                for remote in &manifest.remote_include {
                                    total_components += 1;
                                    let should_include = if remote.id == "default" {
                                        !remote.optional
                                    } else if !remote.optional {
                                        true
                                    } else {
                                        enabled_features.read().contains(&remote.id)
                                    };
                                    
                                    if should_include {
                                        enabled_components += 1;
                                    }
                                }
                                
                                rsx! { "{enabled_components}/{total_components} components enabled" }
                            } else {
                                rsx! { "Loading components..." }
                            }
                        }
                    }
                }
                
                // Centered expand/collapse button
                button { 
                    class: "expand-collapse-button",
                    onclick: move |_| {
                        let current_expanded = *features_expanded.read();
                        features_expanded.set(!current_expanded);
                    },
                    
                    if *features_expanded.read() {
                        span { class: "button-icon collapse-icon", "▲" }
                        "Collapse Features"
                    } else {
                        span { class: "button-icon expand-icon", "▼" }
                        "Expand Features"
                    }
                }
                
                // Collapsible content
                div { 
                    class: if *features_expanded.read() {
                        "optional-features-content expanded"
                    } else {
                        "optional-features-content"
                    },
                    
                    // Search filter
                    div { class: "feature-filter-container",
                        span { class: "feature-filter-icon", "🔍" }
                        input {
                            class: "feature-filter",
                            placeholder: "Search for features...",
                            value: "{filter_text}",
                            oninput: move |evt| filter_text.set(evt.value().clone()),
                        }
                        
                        if !filter_text.read().is_empty() {
                            button {
                                class: "feature-filter-clear",
                                onclick: move |_| filter_text.set(String::new()),
                                "×"
                            }
                        }
                    }
                    
                    // Features content
                    {
                        if let Some(manifest) = &universal_manifest_for_render {
                            render_all_features_sections(
                                manifest.clone(),
                                enabled_features.clone(),
                                filter_text.clone(),
                                toggle_feature
                            )
                        } else {
                            rsx! {
                                div { class: "loading-container",
                                    div { class: "loading-spinner" }
                                    div { class: "loading-text", "Loading features..." }
                                }
                            }
                        }
                    }
                }
            }
        } 
    }
}

// FIXED: Enhanced feature rendering with proper include/remote include handling
fn render_all_features_sections(
    manifest: UniversalManifest,
    enabled_features: Signal<Vec<String>>,
    filter_text: Signal<String>,
    toggle_feature: impl FnMut(String) + Clone + 'static,
) -> Element {
    let filter = filter_text.read().to_lowercase();
    
    // FIXED: Collect ALL components including includes and remote includes
    let mut all_components = Vec::new();
    all_components.extend(manifest.mods.iter().cloned());
    all_components.extend(manifest.shaderpacks.iter().cloned());
    all_components.extend(manifest.resourcepacks.iter().cloned());
    
    // FIXED: Convert includes to ModComponent format properly
    for include in &manifest.include {
        if !include.id.is_empty() && include.id != "default" {
            all_components.push(ModComponent {
                id: include.id.clone(),
                name: include.name.clone().unwrap_or_else(|| {
                    // Extract name from location
                    if include.location.contains('/') {
                        include.location.split('/').last().unwrap_or(&include.location).to_string()
                    } else {
                        include.location.clone()
                    }
                }),
                description: Some(format!("Configuration: {}", include.location)),
                source: "include".to_string(),
                location: include.location.clone(),
                version: "1.0".to_string(),
                path: None,
                optional: include.optional,
                default_enabled: include.default_enabled,
                authors: include.authors.clone().unwrap_or_default(),
                category: Some("Configuration".to_string()),
                dependencies: None,
                incompatibilities: None,
                ignore_update: include.ignore_update,
            });
        }
    }
    
    // FIXED: Convert remote includes to ModComponent format properly
    for remote in &manifest.remote_include {
        if remote.id != "default" {
            all_components.push(ModComponent {
                id: remote.id.clone(),
                name: remote.name.clone().unwrap_or_else(|| {
                    remote.id.replace('_', " ").replace('-', " ")
                }),
                description: remote.description.clone().or_else(|| {
                    Some(format!("Remote content: {}", remote.location))
                }),
                source: "remote_include".to_string(),
                location: remote.location.clone(),
                version: remote.version.clone(),
                path: remote.path.as_ref().map(|p| std::path::PathBuf::from(p)),
                optional: remote.optional,
                default_enabled: remote.default_enabled,
                authors: remote.authors.clone(),
                category: remote.category.clone().or_else(|| Some("Remote Content".to_string())),
                dependencies: remote.dependencies.clone(),
                incompatibilities: None,
                ignore_update: remote.ignore_update,
            });
        }
    }
    
    debug!("Total components after includes/remote includes: {}", all_components.len());
    
    // Filter components
    let filtered_components = if filter.is_empty() {
        all_components
    } else {
        all_components.into_iter()
            .filter(|comp| {
                let name_match = comp.name.to_lowercase().contains(&filter);
                let desc_match = comp.description.as_ref()
                    .map_or(false, |desc| desc.to_lowercase().contains(&filter));
                let category_match = comp.category.as_ref()
                    .map_or(false, |cat| cat.to_lowercase().contains(&filter));
                name_match || desc_match || category_match
            })
            .collect()
    };
    
    // FIXED: Separate into included (default/required) and optional
    let (included_components, optional_components): (Vec<_>, Vec<_>) = filtered_components
        .into_iter()
        .partition(|comp| {
            // Component is included if it's default_enabled AND not optional, OR if it has id "default"
            comp.id == "default" || (comp.default_enabled && !comp.optional)
        });
    
    debug!("Included components: {}", included_components.len());
    debug!("Optional components: {}", optional_components.len());
    
    // Group optional components by category
    let mut categories: std::collections::BTreeMap<String, Vec<ModComponent>> = std::collections::BTreeMap::new();
    for component in optional_components {
        let category = component.category.clone().unwrap_or_else(|| {
            // FIXED: Better category assignment based on source
            match component.source.as_str() {
                "include" => "Configuration".to_string(),
                "remote_include" => "Remote Content".to_string(),
                "modrinth" => {
                    if component.location.contains("shader") {
                        "Shaders".to_string()
                    } else if component.location.contains("resource") {
                        "Resource Packs".to_string()
                    } else {
                        "Mods".to_string()
                    }
                },
                _ => "Other".to_string(),
            }
        });
        categories.entry(category).or_insert_with(Vec::new).push(component);
    }
    
    debug!("Categories: {:?}", categories.keys().collect::<Vec<_>>());
    
    let mut included_expanded = use_signal(|| false);
    let mut expanded_categories = use_signal(|| Vec::<String>::new());
    
    // Check for no results
    let no_results = categories.is_empty() && included_components.is_empty() && !filter.is_empty();
    
    if no_results {
        return rsx! {
            div { class: "no-search-results",
                "No features found matching '{filter}'. Try a different search term."
            }
        };
    }
    
    rsx! {
        div { class: "feature-categories",
            // Included Features Section
            if !included_components.is_empty() {
                div { class: "feature-category",
                    div { 
                        class: "category-header",
                        onclick: move |_| {
                            included_expanded.with_mut(|expanded| *expanded = !*expanded);
                        },
                        
                        div { class: "category-title-section",
                            h3 { class: "category-name", "INCLUDED COMPONENTS" }
                            span { class: "category-count included-count", 
                                "{included_components.len()} included" 
                            }
                        }
                        
                        div { 
                            class: if *included_expanded.read() {
                                "category-toggle-indicator expanded"
                            } else {
                                "category-toggle-indicator"
                            },
                            "▼"
                        }
                    }
                    
                    div { 
                        class: if *included_expanded.read() {
                            "category-content expanded"
                        } else {
                            "category-content"
                        },
                        
                        div { class: "feature-cards-grid",
                            for component in included_components {
                                {
                                    rsx! {
                                        div { 
                                            class: "feature-card feature-included",
                                            
                                            div { class: "feature-card-header",
                                                h3 { class: "feature-card-title", "{component.name}" }
                                                
                                                span {
                                                    class: "feature-toggle-button included-component",
                                                    "Included"
                                                }
                                            }
                                            
                                            if let Some(description) = &component.description {
                                                div { class: "feature-card-description", "{description}" }
                                            }
                                            
                                            if !component.authors.is_empty() {
                                                div { class: "feature-authors",
                                                    "By: ",
                                                    for (i, author) in component.authors.iter().enumerate() {
                                                        {
                                                            let is_last = i == component.authors.len() - 1;
                                                            rsx! {
                                                                a {
                                                                    class: "author-link",
                                                                    href: "{author.link}",
                                                                    target: "_blank",
                                                                    "{author.name}"
                                                                }
                                                                if !is_last {
                                                                    ", "
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
            
            // Optional Features by Category
            for (category_name, components) in categories {
                {
                    let category_key = category_name.clone();
                    let is_expanded = expanded_categories.read().contains(&category_key) 
                                     || !filter.is_empty();
                    
                    let enabled_count = enabled_features.read().iter()
                        .filter(|id| components.iter().any(|comp| &comp.id == *id))
                        .count();
                    
                    let are_all_enabled = !components.is_empty() && enabled_count == components.len();
                    
                    rsx! {
                        div { class: "feature-category",
                            div { 
                                class: "category-header",
                                onclick: {
                                    let category_key = category_key.clone();
                                    move |_| {
                                        expanded_categories.with_mut(|cats| {
                                            if cats.contains(&category_key) {
                                                cats.retain(|c| c != &category_key);
                                            } else {
                                                cats.push(category_key.clone());
                                            }
                                        });
                                    }
                                },
                                
                                div { class: "category-title-section",
                                    h3 { class: "category-name", "{category_name}" }
                                    span { class: "category-count", "{enabled_count}/{components.len()}" }
                                }
                                
                                // Toggle all button
                                {
                                    let components_clone = components.clone();
                                    let mut enabled_features = enabled_features.clone();
                                    
                                    rsx! {
                                        button {
                                            class: if are_all_enabled {
                                                "category-toggle-all toggle-disable"
                                            } else {
                                                "category-toggle-all toggle-enable"
                                            },
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                
                                                enabled_features.with_mut(|features| {
                                                    if are_all_enabled {
                                                        for comp in &components_clone {
                                                            if comp.id != "default" && comp.optional {
                                                                features.retain(|id| id != &comp.id);
                                                            }
                                                        }
                                                    } else {
                                                        for comp in &components_clone {
                                                            if comp.id != "default" && comp.optional && !features.contains(&comp.id) {
                                                                features.push(comp.id.clone());
                                                            }
                                                        }
                                                    }
                                                });
                                            },
                                            
                                            if are_all_enabled {
                                                "Disable All"
                                            } else if enabled_count > 0 {
                                                "Enable All"
                                            } else {
                                                "Enable All"
                                            }
                                        }
                                    }
                                }
                                
                                // Expand/collapse indicator
                                div { 
                                    class: if is_expanded {
                                        "category-toggle-indicator expanded"
                                    } else {
                                        "category-toggle-indicator"
                                    },
                                    "▼"
                                }
                            }
                            
                            // Category content
                            div { 
                                class: if is_expanded {
                                    "category-content expanded"
                                } else {
                                    "category-content"
                                },
                                
                                div { class: "feature-cards-grid",
                                    for component in components {
                                        {
                                            let component_id = component.id.clone();
                                            let is_enabled = enabled_features.read().contains(&component_id);
                                            let mut toggle_func = toggle_feature.clone();
                                            
                                            rsx! {
                                                div { 
                                                    class: if is_enabled {
                                                        "feature-card feature-enabled"
                                                    } else {
                                                        "feature-card feature-disabled"
                                                    },
                                                    
                                                    div { class: "feature-card-header",
                                                        h3 { class: "feature-card-title", "{component.name}" }
                                                        
                                                        if component.id == "default" || !component.optional {
                                                            span {
                                                                class: "feature-toggle-button enabled default-component",
                                                                style: "cursor: default; opacity: 0.8;",
                                                                "Included"
                                                            }
                                                        } else {
                                                            label {
                                                                class: if is_enabled {
                                                                    "feature-toggle-button enabled"
                                                                } else {
                                                                    "feature-toggle-button disabled"
                                                                },
                                                                onclick: move |_| {
                                                                    toggle_func(component_id.clone());
                                                                },
                                                                
                                                                if is_enabled {
                                                                    "Enabled"
                                                                } else {
                                                                    "Disabled"
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    if let Some(description) = &component.description {
                                                        div { class: "feature-card-description", "{description}" }
                                                    }
                                                    
                                                    if let Some(deps) = &component.dependencies {
                                                        if !deps.is_empty() {
                                                            div { class: "feature-dependencies",
                                                                "Requires: ", 
                                                                span { class: "dependency-list", 
                                                                    {deps.join(", ")}
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    if !component.authors.is_empty() {
                                                        div { class: "feature-authors",
                                                            "By: ",
                                                            for (i, author) in component.authors.iter().enumerate() {
                                                                {
                                                                    let is_last = i == component.authors.len() - 1;
                                                                    rsx! {
                                                                        a {
                                                                            class: "author-link",
                                                                            href: "{author.link}",
                                                                            target: "_blank",
                                                                            "{author.name}"
                                                                        }
                                                                        if !is_last {
                                                                            ", "
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
                }
            }
        }
    }
}
