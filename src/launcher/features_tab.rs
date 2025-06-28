use dioxus::prelude::*;
use crate::universal::{ModComponent, UniversalManifest};
use crate::preset::{Preset, find_preset_by_id};
use log::debug;

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
    let installation_id_for_apply = installation_id.clone();
    let installation_id_for_toggle = installation_id.clone();
    
    // Handle changing a preset
    let apply_preset = move |preset_id: String| {
        if let Some(preset) = find_preset_by_id(&presets_for_closure, &preset_id) {
            // Update enabled features
            enabled_features.set(preset.enabled_features.clone());
            
            // Mark as selected
            selected_preset.set(Some(preset_id.clone()));
            
            // Store the preset info in the installation
            if let Ok(mut installation) = crate::installation::load_installation(&installation_id_for_apply) {
                installation.base_preset_id = Some(preset.id.clone());
                installation.base_preset_version = preset.preset_version.clone();
                installation.custom_features.clear();
                installation.removed_features.clear();
                let _ = installation.save();
            }
        }
    };
    
    // Clone presets again for toggle_feature
    let presets_for_toggle = presets.clone();
    let universal_manifest_for_toggle = universal_manifest.clone();
    
    // Handle toggling a feature with dependency checking
    let toggle_feature = move |feature_id: String| {
        // Use the cloned version inside the closure
        let manifest_for_deps = universal_manifest_for_toggle.clone();
        
        enabled_features.with_mut(|features| {
            let is_enabling = !features.contains(&feature_id);
            
            if is_enabling {
                // Add the feature
                if !features.contains(&feature_id) {
                    features.push(feature_id.clone());
                }
                
                // Check for dependencies and enable them too
                if let Some(manifest) = &manifest_for_deps {
                    // Check all component types for the feature being enabled
                    let all_components: Vec<&ModComponent> = manifest.mods.iter()
                        .chain(manifest.shaderpacks.iter())
                        .chain(manifest.resourcepacks.iter())
                        .collect();
                    
                    // Find the component being enabled
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
                
                // Check if any enabled features depend on this one
                if let Some(manifest) = &manifest_for_deps {
                    let all_components: Vec<&ModComponent> = manifest.mods.iter()
                        .chain(manifest.shaderpacks.iter())
                        .chain(manifest.resourcepacks.iter())
                        .collect();
                    
                    // Find features that depend on the one being disabled
                    let dependent_features: Vec<String> = all_components.iter()
                        .filter(|c| {
                            features.contains(&c.id) && 
                            c.dependencies.as_ref().map_or(false, |deps| deps.contains(&feature_id))
                        })
                        .map(|c| c.id.clone())
                        .collect();
                    
                    // Also disable dependent features
                    for dep_feat in dependent_features {
                        debug!("Auto-disabling dependent feature: {} (depends on {})", dep_feat, feature_id);
                        features.retain(|id| id != &dep_feat);
                    }
                }
            }
        });

        if let Ok(mut installation) = crate::installation::load_installation(&installation_id_for_toggle) {
            if let Some(base_preset_id) = &installation.base_preset_id {
                if let Some(base_preset) = find_preset_by_id(&presets_for_toggle, base_preset_id) {
                    // Check if this feature was in the original preset
                    let was_in_preset = base_preset.enabled_features.contains(&feature_id);
                    let is_enabled = enabled_features.read().contains(&feature_id);
                    
                    if was_in_preset && !is_enabled {
                        // Feature was removed from preset
                        if !installation.removed_features.contains(&feature_id) {
                            installation.removed_features.push(feature_id.clone());
                        }
                        installation.custom_features.retain(|id| id != &feature_id);
                    } else if !was_in_preset && is_enabled {
                        // Feature was added to preset
                        if !installation.custom_features.contains(&feature_id) {
                            installation.custom_features.push(feature_id.clone());
                        }
                        installation.removed_features.retain(|id| id != &feature_id);
                    }
                    
                    let _ = installation.save();
                }
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
    
    // Return the rsx! directly without semicolon
    rsx! {
        div { class: "features-tab",
            // PRESETS section header
            div { class: "section-divider with-title", 
                span { class: "divider-title", "PRESETS" }
            }
            
            p { class: "section-description", 
                "Choose a preset configuration or customize individual features below."
            }
            
            // Presets grid - ONLY ONE RENDERING BLOCK
            div { class: "presets-grid",
                // Custom preset (no preset selected)
                div { 
                    class: if selected_preset.read().is_none() {
                        "preset-card selected"
                    } else {
                        "preset-card"
                    },
                    // Apply custom preset background if available
                    style: if let Some(preset) = custom_preset {
                        if let Some(bg) = &preset.background {
                            format!("background-image: url('{}'); background-size: cover; background-position: center;", bg)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    },
                    onclick: move |_| {
                        // When selecting custom preset, ensure default features are included
                        enabled_features.with_mut(|features| {
                            // Always ensure "default" is present
                            if !features.contains(&"default".to_string()) {
                                features.insert(0, "default".to_string());
                            }
                            
                            // Add any default-enabled features from the universal manifest if available
                            if let Some(manifest) = &universal_manifest {
                                for component in &manifest.mods {
                                    if component.default_enabled && !features.contains(&component.id) {
                                        features.push(component.id.clone());
                                    }
                                }
                                for component in &manifest.shaderpacks {
                                    if component.default_enabled && !features.contains(&component.id) {
                                        features.push(component.id.clone());
                                    }
                                }
                                for component in &manifest.resourcepacks {
                                    if component.default_enabled && !features.contains(&component.id) {
                                        features.push(component.id.clone());
                                    }
                                }
                            }
                        });
                        
                        selected_preset.set(None);
                    },
                    
                    div { class: "preset-card-overlay" }
                    
                    div { class: "preset-card-content",
                        h4 { "CUSTOM OVERHAUL" }
                        p { "Start with your current selection and customize everything yourself." }
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
                                
                                // Select/Selected button with comprehensive inline styling
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
                
                // Features count badge
                div { class: "features-count-container",
                    span { class: "features-count-badge",
                        {
                            if let Some(manifest) = &universal_manifest {
                                // Get all optional components
                                let optional_mods = manifest.mods.iter()
                                    .filter(|m| m.optional)
                                    .count();
                                    
                                let optional_shaderpacks = manifest.shaderpacks.iter()
                                    .filter(|m| m.optional)
                                    .count();
                                    
                                let optional_resourcepacks = manifest.resourcepacks.iter()
                                    .filter(|m| m.optional)
                                    .count();
                                    
                                let optional_includes = manifest.include.iter()
                                    .filter(|i| i.optional && !i.id.is_empty())
                                    .count();
                                    
                                // Calculate total features
                                let total_features = optional_mods + optional_shaderpacks + optional_resourcepacks + optional_includes;
                                
                                // Calculate enabled features (minus the default)
                                let enabled_count = enabled_features.read().iter()
                                    .filter(|id| *id != "default")
                                    .count();
                                
                                rsx! { "{enabled_count}/{total_features} features enabled" }
                            } else {
                                rsx! { "Loading features..." }
                            }
                        }
                    }
                }
                
                // Centered expand/collapse button with improved styling
                button { 
                    class: "expand-collapse-button",
                    onclick: move |_| {
                        let current_expanded = *features_expanded.read();
                        features_expanded.set(!current_expanded);
                    },
                    
                    // Icon and text change based on state
                    if *features_expanded.read() {
                        // Collapse state
                        span { class: "button-icon collapse-icon", "▲" }
                        "Collapse Features"
                    } else {
                        // Expand state
                        span { class: "button-icon expand-icon", "▼" }
                        "Expand Features"
                    }
                }
                
                // Collapsible content - search INSIDE this section
                div { 
                    class: if *features_expanded.read() {
                        "optional-features-content expanded"
                    } else {
                        "optional-features-content"
                    },
                    
                    // Search filter at the top of expanded features section
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
                    
                    // Features content - only render if we have a universal manifest
                    {
                        if let Some(manifest) = &universal_manifest {
                            // Get all optional mods
                            let optional_mods: Vec<ModComponent> = manifest.mods.iter()
                                .filter(|m| m.optional)
                                .cloned()
                                .collect();
                            
                            // Get all optional shaderpacks and resourcepacks too
                            let optional_shaderpacks: Vec<ModComponent> = manifest.shaderpacks.iter()
                                .filter(|m| m.optional)
                                .cloned()
                                .collect();
                            
                            let optional_resourcepacks: Vec<ModComponent> = manifest.resourcepacks.iter()
                                .filter(|m| m.optional)
                                .cloned()
                                .collect();
                            
                            // Combine all optional components
                            let mut all_components = Vec::new();
                            all_components.extend(optional_mods);
                            all_components.extend(optional_shaderpacks);
                            all_components.extend(optional_resourcepacks);
                            
                            // Get includes from manifest
                            let includes = manifest.include.clone();
                            
                            // NEW: Get included (default-enabled) components for the top section
                            let included_mods: Vec<ModComponent> = manifest.mods.iter()
                                .filter(|m| m.default_enabled && m.optional)
                                .cloned()
                                .collect();
                                
                            let included_shaderpacks: Vec<ModComponent> = manifest.shaderpacks.iter()
                                .filter(|m| m.default_enabled && m.optional)
                                .cloned()
                                .collect();
                                
                            let included_resourcepacks: Vec<ModComponent> = manifest.resourcepacks.iter()
                                .filter(|m| m.default_enabled && m.optional)
                                .cloned()
                                .collect();
                                
                            // Combine all included components
                            let mut included_components = Vec::new();
                            included_components.extend(included_mods);
                            included_components.extend(included_shaderpacks);
                            included_components.extend(included_resourcepacks);
                            
                            // First render the included features section if there are any
                            if !included_components.is_empty() {
                                rsx! {
                                    // Included Features Section (always visible)
                                    div { class: "feature-category",
                                        // Category header
                                        div { class: "category-header",
                                            div { class: "category-title-section",
                                                h3 { class: "category-name", "✓ Included Components" }
                                                span { class: "category-count included-count", 
                                                    "{included_components.len()} included" 
                                                }
                                            }
                                        }
                                        
                                        // Category content (always expanded)
                                        div { class: "category-content expanded",
                                            // Feature cards grid
                                            div { class: "feature-cards-grid",
                                                for component in included_components {
                                                    {
                                                        let component_id = component.id.clone();
                                                        let is_enabled = enabled_features.read().contains(&component_id);
                                                        
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
                                                                
                                                                // Description display
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
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Display features by category with includes
                            render_features_by_category(all_components, enabled_features.clone(), filter_text.clone(), toggle_feature)  
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

// Helper function to render features by category - unchanged
fn render_features_by_category(
    components: Vec<ModComponent>,
    enabled_features: Signal<Vec<String>>,
    filter_text: Signal<String>,
    toggle_feature: impl FnMut(String) + Clone + 'static,
) -> Element {
    // Apply current filter
    let filter = filter_text.read().to_lowercase();
    let filtered_components = if filter.is_empty() {
        components
    } else {
        components.into_iter()
            .filter(|comp| {
                let name_match = comp.name.to_lowercase().contains(&filter);
                let desc_match = comp.description.as_ref()
                    .map_or(false, |desc| desc.to_lowercase().contains(&filter));
                name_match || desc_match
            })
            .collect()
    };
    
    // Group components by category
    let mut categories: std::collections::BTreeMap<String, Vec<ModComponent>> = std::collections::BTreeMap::new();
    
    // Group components by their categories
    for component in filtered_components {
        // Skip if component is default_enabled as these are shown separately
        if component.default_enabled {
            continue;
        }
        
        let category = component.category.clone().unwrap_or_else(|| "Uncategorized".to_string());
        categories.entry(category).or_insert_with(Vec::new).push(component);
    }
    
    // Create a signal to track expanded categories
    let mut expanded_categories = use_signal(|| Vec::<String>::new());
    
    // Check if no results match the filter
    let no_results = categories.is_empty() && !filter.is_empty();
    
    // If no results found
    if no_results {
        return rsx! {
            div { class: "no-search-results",
                "No features found matching '{filter}'. Try a different search term."
            }
        };
    }
    
    // Render categories
    rsx! {
        div { class: "feature-categories",
            for (category_name, components) in categories {
                {
                    let category_key = category_name.clone();
                    let is_expanded = expanded_categories.read().contains(&category_key) 
                                 || filter.is_empty() == false; // Auto-expand when filtering
                    
                    // Calculate how many components are enabled (excluding default/included)
                    let optional_components: Vec<_> = components.iter()
                        .filter(|comp| comp.id != "default" && comp.optional)
                        .collect();
                    let enabled_count = enabled_features.read().iter()
                        .filter(|id| optional_components.iter().any(|comp| &comp.id == *id))
                        .count();
                    
                    let are_all_enabled = !optional_components.is_empty() && enabled_count == optional_components.len();
                    
                    rsx! {
                        div { class: "feature-category",
                            // Category header - ENTIRE HEADER IS CLICKABLE
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
                                
                                // Toggle all button - has separate click handler
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
                                                // Stop propagation to prevent header's click handler
                                                evt.stop_propagation();
                                                
                                                // Toggle all in category (excluding default/included)
                                                enabled_features.with_mut(|features| {
                                                    if are_all_enabled {
                                                        // Disable all optional components
                                                        for comp in &components_clone {
                                                            if comp.id != "default" && comp.optional {
                                                                features.retain(|id| id != &comp.id);
                                                            }
                                                        }
                                                    } else {
                                                        // Enable all optional components
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
                                
                                // Expand/collapse indicator - larger, more visible
                                div { 
                                    class: if is_expanded {
                                        "category-toggle-indicator expanded"
                                    } else {
                                        "category-toggle-indicator"
                                    },
                                    "▼"
                                }
                            }
                            
                            // Category content (expandable)
                            div { 
                                class: if is_expanded {
                                    "category-content expanded"
                                } else {
                                    "category-content"
                                },
                                
                                // Feature cards grid
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
                                                        
                                                        // Special handling for default/included components
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
                                                    
                                                    // Description display
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
