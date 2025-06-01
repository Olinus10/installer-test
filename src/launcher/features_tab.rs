use dioxus::prelude::*;
use crate::universal::{ModComponent, UniversalManifest, IncludeComponent, RemoteIncludeComponent};
use crate::preset::{Preset, find_preset_by_id};

#[component]
pub fn FeaturesTab(
    universal_manifest: Option<UniversalManifest>,
    presets: Vec<Preset>,
    enabled_features: Signal<Vec<String>>,
    selected_preset: Signal<Option<String>>,
    filter_text: Signal<String>,
) -> Element {
    // Clone presets to avoid ownership issues in closure
    let presets_for_closure = presets.clone();
    
    // Handle changing a preset
    let apply_preset = move |preset_id: String| {
        if let Some(preset) = find_preset_by_id(&presets_for_closure, &preset_id) {
            // Update enabled features
            enabled_features.set(preset.enabled_features.clone());
            
            // Mark as selected
            selected_preset.set(Some(preset_id));
        }
    };
    
    // Handle toggling a feature
    let toggle_feature = move |feature_id: String| {
        enabled_features.with_mut(|features| {
            if features.contains(&feature_id) {
                features.retain(|id| id != &feature_id);
            } else {
                features.push(feature_id.clone());
            }
        });
        
        // Clear selected preset when features are manually changed
        selected_preset.set(None);
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
                        selected_preset.set(None);
                    },
                    
                    div { class: "preset-card-overlay" }
                    
                    div { class: "preset-card-content",
                        h4 { "CUSTOM OVERHAUL" }
                        p { "Start with your current selection and customize everything yourself." }
                    }
                    
                    // Select/Selected button
                    {
                        let is_selected = selected_preset.read().is_none();
                        
                        rsx! {
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
                                
                                if is_selected {
                                    "SELECTED"
                                } else {
                                    "SELECT"
                                }
                            }
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
                            if let Some(manifest) = &universal_manifest {
                                // Get all components (both optional and default)
                                let all_mods = manifest.mods.len();
                                let all_shaderpacks = manifest.shaderpacks.len();
                                let all_resourcepacks = manifest.resourcepacks.len();
                                let all_includes = manifest.includes.len();
                                let all_remote_includes = manifest.remote_includes.len();
                                
                                let total_features = all_mods + all_shaderpacks + all_resourcepacks + all_includes + all_remote_includes;
                                let enabled_count = enabled_features.read().len();
                                
                                rsx! { "{enabled_count}/{total_features} features" }
                            } else {
                                rsx! { "Loading features..." }
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
                        if let Some(manifest) = &universal_manifest {
                            render_all_features_by_category(manifest, enabled_features.clone(), filter_text.clone(), toggle_feature)
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

// NEW: Helper function to render ALL features (both optional and default) by category
fn render_all_features_by_category(
    manifest: &UniversalManifest,
    enabled_features: Signal<Vec<String>>,
    filter_text: Signal<String>,
    toggle_feature: impl FnMut(String) + Clone + 'static,
) -> Element {
    // Collect all components into a unified structure
    let mut all_components = Vec::new();
    
    // Add mods
    for component in &manifest.mods {
        all_components.push(UnifiedComponent {
            id: component.id.clone(),
            name: component.name.clone(),
            description: component.description.clone(),
            category: component.category.clone().unwrap_or_else(|| "Uncategorized".to_string()),
            optional: component.optional,
            default_enabled: component.default_enabled,
            authors: component.authors.clone(),
            dependencies: component.dependencies.clone(),
            component_type: "Mod".to_string(),
        });
    }
    
    // Add shaderpacks
    for component in &manifest.shaderpacks {
        all_components.push(UnifiedComponent {
            id: component.id.clone(),
            name: component.name.clone(),
            description: component.description.clone(),
            category: component.category.clone().unwrap_or_else(|| "Uncategorized".to_string()),
            optional: component.optional,
            default_enabled: component.default_enabled,
            authors: component.authors.clone(),
            dependencies: component.dependencies.clone(),
            component_type: "Shader".to_string(),
        });
    }
    
    // Add resourcepacks
    for component in &manifest.resourcepacks {
        all_components.push(UnifiedComponent {
            id: component.id.clone(),
            name: component.name.clone(),
            description: component.description.clone(),
            category: component.category.clone().unwrap_or_else(|| "Uncategorized".to_string()),
            optional: component.optional,
            default_enabled: component.default_enabled,
            authors: component.authors.clone(),
            dependencies: component.dependencies.clone(),
            component_type: "Resource Pack".to_string(),
        });
    }
    
    // Add includes
    for component in &manifest.includes {
        all_components.push(UnifiedComponent {
            id: component.id.clone(),
            name: component.name.clone(),
            description: component.description.clone(),
            category: component.category.clone().unwrap_or_else(|| "CORE".to_string()),
            optional: component.optional,
            default_enabled: component.default_enabled,
            authors: component.authors.clone(),
            dependencies: component.dependencies.clone(),
            component_type: "Config/Data".to_string(),
        });
    }
    
    // Add remote includes
    for component in &manifest.remote_includes {
        all_components.push(UnifiedComponent {
            id: component.id.clone(),
            name: component.name.clone(),
            description: component.description.clone(),
            category: component.category.clone().unwrap_or_else(|| "CORE".to_string()),
            optional: component.optional,
            default_enabled: component.default_enabled,
            authors: component.authors.clone(),
            dependencies: component.dependencies.clone(),
            component_type: "Remote Config".to_string(),
        });
    }
    
    // Apply filter
    let filter = filter_text.read().to_lowercase();
    let filtered_components = if filter.is_empty() {
        all_components
    } else {
        all_components.into_iter()
            .filter(|comp| {
                let name_match = comp.name.to_lowercase().contains(&filter);
                let desc_match = comp.description.as_ref()
                    .map_or(false, |desc| desc.to_lowercase().contains(&filter));
                name_match || desc_match
            })
            .collect()
    };
    
    // Group by category, separating optional and default features
    let mut optional_categories: std::collections::BTreeMap<String, Vec<UnifiedComponent>> = std::collections::BTreeMap::new();
    let mut default_categories: std::collections::BTreeMap<String, Vec<UnifiedComponent>> = std::collections::BTreeMap::new();
    
    for component in filtered_components {
        let category = component.category.clone();
        if component.optional {
            optional_categories.entry(category).or_insert_with(Vec::new).push(component);
        } else {
            default_categories.entry(category).or_insert_with(Vec::new).push(component);
        }
    }
    
    let mut expanded_categories = use_signal(|| Vec::<String>::new());
    
    // Check if no results match the filter
    let no_results = optional_categories.is_empty() && default_categories.is_empty() && !filter.is_empty();
    
    if no_results {
        return rsx! {
            div { class: "no-search-results",
                "No features found matching '{filter}'. Try a different search term."
            }
        };
    }
    
    rsx! {
        div { class: "feature-categories",
            // Show default/core features first
            if !default_categories.is_empty() {
                div { class: "feature-category-section",
                    h2 { class: "category-section-title", "CORE FEATURES" }
                    p { class: "category-section-description", 
                        "These features are included by default and essential for the modpack to function properly."
                    }
                    
                    for (category_name, components) in default_categories {
                        {
                            let category_key = format!("default-{}", category_name);
                            let is_expanded = expanded_categories.read().contains(&category_key) || !filter.is_empty();
                            
                            rsx! {
                                div { class: "feature-category default-category",
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
                                            span { class: "category-count default-count", "{components.len()} core" }
                                        }
                                        
                                        div { 
                                            class: if is_expanded {
                                                "category-toggle-indicator expanded"
                                            } else {
                                                "category-toggle-indicator"
                                            },
                                            "▼"
                                        }
                                    }
                                    
                                    div { 
                                        class: if is_expanded {
                                            "category-content expanded"
                                        } else {
                                            "category-content"
                                        },
                                        
                                        div { class: "feature-cards-grid",
                                            for component in components {
                                                render_feature_card(component, enabled_features.clone(), toggle_feature.clone(), true)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Show optional features
            if !optional_categories.is_empty() {
                div { class: "feature-category-section",
                    h2 { class: "category-section-title", "OPTIONAL FEATURES" }
                    p { class: "category-section-description", 
                        "These features can be enabled or disabled to customize your experience."
                    }
                    
                    for (category_name, components) in optional_categories {
                        {
                            let category_key = format!("optional-{}", category_name);
                            let is_expanded = expanded_categories.read().contains(&category_key) || !filter.is_empty();
                            
                            let enabled_count = enabled_features.read().iter()
                                .filter(|id| components.iter().any(|comp| &comp.id == *id))
                                .count();
                            
                            let are_all_enabled = enabled_count == components.len();
                            
                            rsx! {
                                div { class: "feature-category optional-category",
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
                                                                    features.retain(|id| id != &comp.id);
                                                                }
                                                            } else {
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
                                        
                                        div { 
                                            class: if is_expanded {
                                                "category-toggle-indicator expanded"
                                            } else {
                                                "category-toggle-indicator"
                                            },
                                            "▼"
                                        }
                                    }
                                    
                                    div { 
                                        class: if is_expanded {
                                            "category-content expanded"
                                        } else {
                                            "category-content"
                                        },
                                        
                                        div { class: "feature-cards-grid",
                                            for component in components {
                                                render_feature_card(component, enabled_features.clone(), toggle_feature.clone(), false)
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

// Helper struct to unify all component types
#[derive(Clone, Debug)]
struct UnifiedComponent {
    id: String,
    name: String,
    description: Option<String>,
    category: String,
    optional: bool,
    default_enabled: bool,
    authors: Vec<crate::Author>,
    dependencies: Option<Vec<String>>,
    component_type: String,
}

fn render_feature_card(
    component: UnifiedComponent,
    enabled_features: Signal<Vec<String>>,
    mut toggle_feature: impl FnMut(String) + Clone + 'static,
    is_core: bool,
) -> Element {
    let component_id = component.id.clone();
    let is_enabled = enabled_features.read().contains(&component_id);
    
    rsx! {
        div { 
            class: if is_core {
                "feature-card core-feature"
            } else if is_enabled {
                "feature-card feature-enabled"
            } else {
                "feature-card feature-disabled"
            },
            
            div { class: "feature-card-header",
                h3 { class: "feature-card-title", 
                    "{component.name}"
                    span { class: "component-type-badge", "{component.component_type}" }
                }
                
                if !is_core {
                    label {
                        class: if is_enabled {
                            "feature-toggle-button enabled"
                        } else {
                            "feature-toggle-button disabled"
                        },
                        onclick: move |_| {
                            toggle_feature(component_id.clone());
                        },
                        
                        if is_enabled {
                            "Enabled"
                        } else {
                            "Disabled"
                        }
                    }
                } else {
                    span { class: "core-feature-label", "Core" }
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
