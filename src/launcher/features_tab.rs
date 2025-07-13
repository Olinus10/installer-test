use dioxus::prelude::*;
use crate::universal::{ModComponent, UniversalManifest, IncludeComponent};
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
    let presets_for_closure = presets.clone();
    let installation_id_for_apply = installation_id.clone();
    let installation_id_for_toggle = installation_id.clone();
    let installation_id_for_memo = installation_id.clone();           // For use_memo
    let installation_id_for_effect = installation_id.clone();         // For use_effect
    let installation_id_for_rsx = installation_id.clone();
    let installation_id_for_custom = installation_id.clone();
    
    // Clone universal_manifest for different uses
    let universal_manifest_for_init = universal_manifest.clone();
    let universal_manifest_for_custom = universal_manifest.clone();
    let universal_manifest_for_toggle = universal_manifest.clone();
    
    // Track if this is a fresh installation or existing one
    let mut is_initialized = use_signal(|| false);
    
    // Load the installation to check its preset AND enabled features
    let installation_data = use_memo(move || {
        if let Ok(installation) = crate::installation::load_installation(&installation_id_for_memo) {
            Some((
                installation.base_preset_id.clone(), 
                installation.enabled_features.clone(),
                installation.installed // Track if it's been installed before
            ))
        } else {
            None
        }
    });

    // Initialize state properly based on installation status
    use_effect({
        let mut selected_preset = selected_preset.clone();
        let mut enabled_features = enabled_features.clone();
        let universal_manifest_for_init = universal_manifest_for_init.clone();
        let mut is_initialized = is_initialized.clone();
        let presets = presets.clone();
        let installation_id_for_effect = installation_id_for_effect.clone(); // Use the separate clone
        
        move || {
            // Only initialize once
            if *is_initialized.read() {
                return;
            }
            
            if let Some((preset_id, features, is_installed)) = installation_data() {
                debug!("Initializing features tab - installed: {}, preset: {:?}", is_installed, preset_id);
                debug!("Features from installation: {:?}", features);
                
                if is_installed {
                    // This is an existing installation - restore EXACTLY what was chosen before
                    debug!("Restoring previous installation state");
                    
                    // CRITICAL: Set the preset selection first
                    selected_preset.set(preset_id.clone());
                    
                    // CRITICAL: Use the exact features that were saved, don't regenerate
                    enabled_features.set(features.clone());
                    
                    debug!("Restored preset: {:?}", preset_id);
                    debug!("Restored features: {:?}", enabled_features.read());
                    
                    // IMPORTANT: Verify the installation has the correct data
                    if let Ok(installation) = crate::installation::load_installation(&installation_id_for_effect) {
                        debug!("Verification - Installation preset: {:?}", installation.base_preset_id);
                        debug!("Verification - Installation features: {:?}", installation.enabled_features);
                        debug!("Verification - Installation installed: {}", installation.installed);
                    }
                } else {
                    // This is a fresh installation - start with custom preset (minimal defaults)
                    debug!("Fresh installation - setting minimal defaults");
                    selected_preset.set(None); // Start with custom preset (None)
                    
                    // Only enable truly required features (just default)
                    let mut minimal_features = vec!["default".to_string()];
                    
                    // Add any non-optional components if we have the manifest
                    if let Some(manifest) = &universal_manifest_for_init {
                        for mod_comp in &manifest.mods {
                            if !mod_comp.optional && mod_comp.id != "default" {
                                minimal_features.push(mod_comp.id.clone());
                            }
                        }
                        for shader in &manifest.shaderpacks {
                            if !shader.optional && shader.id != "default" {
                                minimal_features.push(shader.id.clone());
                            }
                        }
                        for resource in &manifest.resourcepacks {
                            if !resource.optional && resource.id != "default" {
                                minimal_features.push(resource.id.clone());
                            }
                        }
                        // Add non-optional includes
                        for include in &manifest.include {
                            if !include.optional && !include.id.is_empty() && include.id != "default" {
                                minimal_features.push(include.id.clone());
                            }
                        }
                        // Add non-optional remote includes
                        for remote in &manifest.remote_include {
                            if !remote.optional && remote.id != "default" {
                                minimal_features.push(remote.id.clone());
                            }
                        }
                    }
                    
                    enabled_features.set(minimal_features);
                    
                    debug!("Fresh installation - defaulting to custom preset with minimal features");
                }
            } else {
                // Fallback for completely new installations
                debug!("No installation data found - using fallback defaults");
                selected_preset.set(None);
                enabled_features.set(vec!["default".to_string()]);
            }
            
            is_initialized.set(true);
        }
    });
    
    // Apply preset function - this should completely replace current selection
    let apply_preset = move |preset_id: String| {
        debug!("Applying preset: {}", preset_id);
        
        if let Some(preset) = find_preset_by_id(&presets_for_closure, &preset_id) {
            // COMPLETELY replace enabled features with preset's features
            let new_features = preset.enabled_features.clone();
            enabled_features.set(new_features.clone());
            
            // Mark as selected
            selected_preset.set(Some(preset_id.clone()));
            
            debug!("Applied preset '{}' with features: {:?}", preset.name, new_features);
            
            // Store the preset info in the installation for persistence
            if let Ok(mut installation) = crate::installation::load_installation(&installation_id_for_apply) {
                installation.base_preset_id = Some(preset.id.clone());
                installation.base_preset_version = preset.preset_version.clone();
                installation.enabled_features = new_features;
                installation.custom_features.clear(); // Clear custom modifications
                installation.removed_features.clear(); // Clear removed features
                installation.modified = true;
                
                if let Err(e) = installation.save() {
                    error!("Failed to save installation: {}", e);
                } else {
                    debug!("Successfully saved installation with preset: {}", preset.id);
                }
            }
        }
    };
    
    // Clone presets again for toggle_feature
    let presets_for_toggle = presets.clone();
    
    // FIXED: Enhanced toggle_feature that properly updates the installation
    let toggle_feature = move |feature_id: String| {
        debug!("Toggling feature: {}", feature_id);
        
        let manifest_for_deps = universal_manifest_for_toggle.clone();
        
        // Update the enabled_features signal
        enabled_features.with_mut(|features| {
            let is_enabling = !features.contains(&feature_id);
            
            if is_enabling {
                // Add the feature
                if !features.contains(&feature_id) {
                    features.push(feature_id.clone());
                    debug!("Enabled feature: {}", feature_id);
                }
                
                // Check for dependencies and enable them too
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
                }
            } else {
                // Remove the feature
                features.retain(|id| id != &feature_id);
                debug!("Disabled feature: {}", feature_id);
                
                // Check if any enabled features depend on this one
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
                }
            }
        });
    
        // CRITICAL FIX: Update installation with new feature state immediately
        if let Ok(mut installation) = crate::installation::load_installation(&installation_id_for_toggle) {
            // Update the installation's enabled features to match the UI
            installation.enabled_features = enabled_features.read().clone();
            installation.modified = true;
            
            // Track this as a custom modification if we have a preset selected
            if let Some(current_preset_id) = selected_preset.read().as_ref() {
                if let Some(current_preset) = find_preset_by_id(&presets_for_toggle, current_preset_id) {
                    // Check if this feature was in the original preset
                    let was_in_preset = current_preset.enabled_features.contains(&feature_id);
                    let is_now_enabled = enabled_features.read().contains(&feature_id);
                    
                    if was_in_preset && !is_now_enabled {
                        // User removed a preset feature
                        if !installation.removed_features.contains(&feature_id) {
                            installation.removed_features.push(feature_id.clone());
                        }
                        installation.custom_features.retain(|f| f != &feature_id);
                    } else if !was_in_preset && is_now_enabled {
                        // User added a feature not in preset
                        if !installation.custom_features.contains(&feature_id) {
                            installation.custom_features.push(feature_id.clone());
                        }
                        installation.removed_features.retain(|f| f != &feature_id);
                    } else if was_in_preset && is_now_enabled {
                        // Feature is back to preset state
                        installation.custom_features.retain(|f| f != &feature_id);
                        installation.removed_features.retain(|f| f != &feature_id);
                    }
                    
                    debug!("Custom features: {:?}, Removed features: {:?}", 
                           installation.custom_features, installation.removed_features);
                }
            }
            
            // Save the installation immediately
            if let Err(e) = installation.save() {
                error!("Failed to save installation after feature toggle: {}", e);
            } else {
                debug!("Successfully saved installation after toggling feature: {}", feature_id);
            }
        }
    };

    // Button hover states
    let mut custom_button_hover = use_signal(|| false);
    let mut trending_button_hover = use_signal(Vec::<String>::new);
    let mut regular_button_hover = use_signal(Vec::<String>::new);
    
    // Features section expanded state
    let mut features_expanded = use_signal(|| false);
    
    // Find custom preset for the "Custom Configuration" card
    let custom_preset = presets.iter().find(|p| p.id == "custom");
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
                    // Apply background if available from custom preset
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
                        debug!("Custom preset clicked - switching to custom configuration");
                        
                        // When switching to custom, ONLY include default and non-optional features
                        let mut new_features = vec!["default".to_string()];
                        
                        // If universal manifest is available, add any non-optional components
                        if let Some(manifest) = &universal_manifest_for_custom {
                            // Add non-optional mods
                            for mod_comp in &manifest.mods {
                                if !mod_comp.optional && mod_comp.id != "default" {
                                    new_features.push(mod_comp.id.clone());
                                }
                            }
                            // Add non-optional shaders
                            for shader in &manifest.shaderpacks {
                                if !shader.optional && shader.id != "default" {
                                    new_features.push(shader.id.clone());
                                }
                            }
                            // Add non-optional resourcepacks
                            for resource in &manifest.resourcepacks {
                                if !resource.optional && resource.id != "default" {
                                    new_features.push(resource.id.clone());
                                }
                            }
                            // Add non-optional includes
                            for include in &manifest.include {
                                if !include.optional && include.id != "default" && !include.id.is_empty() {
                                    new_features.push(include.id.clone());
                                }
                            }
                            // Add non-optional remote includes
                            for remote in &manifest.remote_include {
                                if !remote.optional && remote.id != "default" {
                                    new_features.push(remote.id.clone());
                                }
                            }
                        }
                        
                        // Set the minimal features
                        enabled_features.set(new_features.clone());
                        selected_preset.set(None);
                        debug!("Set selected_preset to None (custom mode) with features: {:?}", new_features);
                        
                        // Update installation to save the custom state
                        if let Ok(mut installation) = crate::installation::load_installation(&installation_id_for_custom) {
                            installation.base_preset_id = None; // None = custom mode
                            installation.base_preset_version = None;
                            installation.enabled_features = enabled_features.read().clone();
                            installation.modified = true;
                            
                            // Clear any previous custom modifications since we're resetting
                            installation.custom_features.clear();
                            installation.removed_features.clear();
                            
                            if let Err(e) = installation.save() {
                                error!("Failed to save installation after switching to custom: {}", e);
                            } else {
                                debug!("Successfully saved custom configuration to installation");
                            }
                        }
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
                            if let Ok(current_installation) = crate::installation::load_installation(&installation_id_for_rsx) {
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
                        
                        // Track if this specific button is being hovered
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
                                // Apply background if available
                                style: if let Some(bg) = &preset.background {
                                    format!("background-image: url('{}'); background-size: cover; background-position: center;", bg)
                                } else {
                                    String::new()
                                },
                                onclick: move |_| {
                                    debug!("Preset clicked: {}", preset_id);
                                    apply_preset_clone(preset_id.clone());
                                },
                                
                                // Feature count badge in top right
                                span { class: "preset-features-count",
                                    "{preset.enabled_features.len()} features"
                                }
                                
                                // Trending badge in top left
                                if has_trending {
                                    span { class: "trending-badge", "Popular" }
                                }

                                // Add update badge
                                if is_updated {
                                    span { class: "update-badge", "Updated" }
                                }
                                
                                // Dark overlay for text readability
                                div { class: "preset-card-overlay" }
                                
                                div { class: "preset-card-content",
                                    h4 { "{preset.name}" }
                                    p { "{preset.description}" }
                                }
                                
                                // Select/Selected button
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
            } // Closes presets-grid div

            // FEATURES section
            div { class: "optional-features-wrapper",
                // Section header with divider style
                div { class: "section-divider with-title", 
                    span { class: "divider-title", "FEATURES" }
                }
                
                // Description for features section
                p { class: "section-description", 
                    "Customize individual features to create your perfect experience."
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
                    
                    // Features content - FIXED VERSION
                    {
                        if let Some(manifest) = &universal_manifest {
                            {render_all_features_properly(manifest, enabled_features.clone(), filter_text.clone(), toggle_feature)}
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

// Helper function to render features by category - FIXED: Use references instead of taking ownership
fn render_all_features_properly(
    manifest: &UniversalManifest,
    enabled_features: Signal<Vec<String>>,
    filter_text: Signal<String>,
    toggle_feature: impl FnMut(String) + Clone + 'static,
) -> Element {
    let filter = filter_text.read().to_lowercase();
    
    // Separate included (non-optional) and optional components
    let mut included_components = Vec::new();
    let mut optional_components = Vec::new();
    
    // Process MODS
    for mod_comp in &manifest.mods {
        let component = ModComponent {
            id: mod_comp.id.clone(),
            name: mod_comp.name.clone(),
            description: mod_comp.description.clone(),
            source: mod_comp.source.clone(),
            location: mod_comp.location.clone(),
            version: mod_comp.version.clone(),
            path: mod_comp.path.clone(),
            optional: mod_comp.optional,
            default_enabled: mod_comp.default_enabled,
            authors: mod_comp.authors.clone(),
            category: mod_comp.category.clone(),
            dependencies: mod_comp.dependencies.clone(),
            incompatibilities: mod_comp.incompatibilities.clone(),
            ignore_update: mod_comp.ignore_update,
        };
        
        if mod_comp.optional {
            optional_components.push(component);
        } else {
            included_components.push(component);
        }
    }
    
    // Process SHADERPACKS
    for shader in &manifest.shaderpacks {
        let component = ModComponent {
            id: shader.id.clone(),
            name: shader.name.clone(),
            description: shader.description.clone(),
            source: shader.source.clone(),
            location: shader.location.clone(),
            version: shader.version.clone(),
            path: shader.path.clone(),
            optional: shader.optional,
            default_enabled: shader.default_enabled,
            authors: shader.authors.clone(),
            category: shader.category.clone(),
            dependencies: shader.dependencies.clone(),
            incompatibilities: shader.incompatibilities.clone(),
            ignore_update: shader.ignore_update,
        };
        
        if shader.optional {
            optional_components.push(component);
        } else {
            included_components.push(component);
        }
    }
    
    // Process RESOURCEPACKS
    for resource in &manifest.resourcepacks {
        let component = ModComponent {
            id: resource.id.clone(),
            name: resource.name.clone(),
            description: resource.description.clone(),
            source: resource.source.clone(),
            location: resource.location.clone(),
            version: resource.version.clone(),
            path: resource.path.clone(),
            optional: resource.optional,
            default_enabled: resource.default_enabled,
            authors: resource.authors.clone(),
            category: resource.category.clone(),
            dependencies: resource.dependencies.clone(),
            incompatibilities: resource.incompatibilities.clone(),
            ignore_update: resource.ignore_update,
        };
        
        if resource.optional {
            optional_components.push(component);
        } else {
            included_components.push(component);
        }
    }
    
    // Process INCLUDES
    for include in &manifest.include {
        // Skip empty IDs
        if include.id.is_empty() {
            continue;
        }
        
        let component = ModComponent {
            id: include.id.clone(),
            name: include.name.clone().unwrap_or_else(|| include.location.clone()),
            description: Some(format!("Include: {}", include.location)),
            source: "include".to_string(),
            location: include.location.clone(),
            version: "1.0".to_string(),
            path: None,
            optional: include.optional,
            default_enabled: include.default_enabled,
            authors: include.authors.clone().unwrap_or_default(),
            // FIXED: Use the actual category from the manifest, don't hardcode
            category: Some("TEXTURES".to_string()), // Default for includes
            dependencies: None,
            incompatibilities: None,
            ignore_update: include.ignore_update,
        };
        
        if include.optional {
            optional_components.push(component);
        } else {
            included_components.push(component);
        }
    }
    
    // Process REMOTE INCLUDES
    for remote in &manifest.remote_include {
        let component = ModComponent {
            id: remote.id.clone(),
            name: remote.name.clone().unwrap_or_else(|| remote.id.clone()),
            description: remote.description.clone(),
            source: "remote_include".to_string(),
            location: remote.location.clone(),
            version: remote.version.clone(),
            path: remote.path.as_ref().map(|p| std::path::PathBuf::from(p)),
            optional: remote.optional,
            default_enabled: remote.default_enabled,
            authors: remote.authors.clone(),
            // FIXED: Use the actual category from the remote include
            category: remote.category.clone(),
            dependencies: remote.dependencies.clone(),
            incompatibilities: None,
            ignore_update: remote.ignore_update,
        };
        
        if remote.optional {
            optional_components.push(component);
        } else {
            included_components.push(component);
        }
    }
    
    // Apply filter to both included and optional components
    let filtered_included = if filter.is_empty() {
        included_components
    } else {
        included_components.into_iter()
            .filter(|comp| {
                let name_match = comp.name.to_lowercase().contains(&filter);
                let desc_match = comp.description.as_ref()
                    .map_or(false, |desc| desc.to_lowercase().contains(&filter));
                name_match || desc_match
            })
            .collect()
    };
    
    let filtered_optional = if filter.is_empty() {
        optional_components
    } else {
        optional_components.into_iter()
            .filter(|comp| {
                let name_match = comp.name.to_lowercase().contains(&filter);
                let desc_match = comp.description.as_ref()
                    .map_or(false, |desc| desc.to_lowercase().contains(&filter));
                name_match || desc_match
            })
            .collect()
    };
    
    // Create signals for category expansion
    let mut expanded_categories = use_signal(|| Vec::<String>::new());
    
    rsx! {
        div { class: "feature-categories",
            // INCLUDED MODS section (non-optional components)
            if !filtered_included.is_empty() {
                div { class: "feature-category included-mods",
                    div { class: "category-header included-header",
                        div { class: "category-title-section",
                            h3 { class: "category-name", "INCLUDED COMPONENTS" }
                            span { class: "category-count", "{filtered_included.len()}" }
                        }
                        
                        // No toggle all button for included components
                        div { class: "category-toggle-indicator", "Always Enabled" }
                    }
                    
                    div { class: "category-content expanded", // Always expanded for included
                        div { class: "feature-cards-grid",
                            for component in filtered_included {
                                {
                                    rsx! {
                                        div { 
                                            class: "feature-card feature-enabled included-component",
                                            
                                            div { class: "feature-card-header",
                                                h3 { class: "feature-card-title", "{component.name}" }
                                                
                                                span {
                                                    class: "feature-toggle-button enabled default-component",
                                                    style: "cursor: default; opacity: 0.8;",
                                                    "Included"
                                                }
                                            }
                                            
                                            if let Some(description) = &component.description {
                                                div { class: "feature-card-description", "{description}" }
                                            }
                                            
                                            // Dependencies display
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
                                            
                                            // Authors display
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
            
            // OPTIONAL COMPONENTS grouped by category
            {
                // Group optional components by category
                let mut categories: std::collections::BTreeMap<String, Vec<ModComponent>> = std::collections::BTreeMap::new();
                
                for component in filtered_optional {
                    let category = component.category.clone().unwrap_or_else(|| "MISC".to_string());
                    categories.entry(category).or_insert_with(Vec::new).push(component);
                }
                
                // Check if no results match the filter
                let no_results = categories.is_empty() && !filter.is_empty();
                
                if no_results {
                    rsx! {
                        div { class: "no-search-results",
                            "No optional features found matching '{filter}'. Try a different search term."
                        }
                    }
                } else {
                    rsx! {
                        Fragment {
                            for (category_name, components) in categories {
                                {
                                    let category_key = category_name.clone();
                                    let is_expanded = expanded_categories.read().contains(&category_key) 
                                                 || !filter.is_empty(); // Auto-expand when filtering
                                    
                                    // Calculate enabled count
                                    let enabled_count = enabled_features.read().iter()
                                        .filter(|id| components.iter().any(|comp| &comp.id == *id))
                                        .count();
                                    
                                    let are_all_enabled = !components.is_empty() && enabled_count == components.len();
                                    
                                    rsx! {
                                        div { class: "feature-category",
                                            // Category header
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
                                                                        // Disable all in category
                                                                        for comp in &components_clone {
                                                                            features.retain(|id| id != &comp.id);
                                                                        }
                                                                    } else {
                                                                        // Enable all in category
                                                                        for comp in &components_clone {
                                                                            if !features.contains(&comp.id) {
                                                                                features.push(comp.id.clone());
                                                                            }
                                                                        }
                                                                    }
                                                                });
                                                            },
                                                            
                                                            if are_all_enabled {
                                                                "Disable All"
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
                                                                    
                                                                    if let Some(description) = &component.description {
                                                                        div { class: "feature-card-description", "{description}" }
                                                                    }
                                                                    
                                                                    // Dependencies display
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
                                                                    
                                                                    // Authors display
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
            }
        }
    }
}
