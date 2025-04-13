// features_tab.rs - Complete fixed version
use dioxus::prelude::*;
use crate::universal::{ModComponent, UniversalManifest};
use crate::preset::{Preset, find_preset_by_id};
// Remove unused import
// use crate::launcher::integrated_features::IntegratedFeatures;

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
    
    // Find custom preset for the "Custom Configuration" card
    let custom_preset = presets.iter().find(|p| p.id == "custom");
    
    rsx! {
        div { class: "features-tab",
            h2 { "Features & Presets" }
            p { "Choose a preset or customize individual features to match your preferences." }
            
            // Add search filter
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
            
            // Presets section
            div { class: "presets-section",
                h3 { "Presets" }
                p { class: "presets-description", 
                    "Choose a preset configuration or customize individual features below."
                }
                
                div { class: "presets-grid",
                    // Custom preset (no preset)
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
                            selected_preset.set(None);
                        },
                        
                        div { class: "preset-card-overlay" }
                        
                        div { class: "preset-card-content",
                            h4 { "Custom Configuration" }
                            p { "Start with your current selection and customize everything yourself." }
                            
                            // Feature count badge
                            span { class: "preset-features-count",
                                "{enabled_features.read().len()} features selected"
                            }
                        }
                    }
                    
                    // Available presets - skip the "custom" preset since we handle it separately
                    for preset in presets.iter().filter(|p| p.id != "custom") {
                        {
                            let preset_id = preset.id.clone();
                            let is_selected = selected_preset.read().as_ref().map_or(false, |id| id == &preset_id);
                            let apply_preset_clone = apply_preset.clone();
                            
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
                                    
                                    // Dark overlay for text readability
                                    div { class: "preset-card-overlay" }
                                    
                                    div { class: "preset-card-content",
                                        // Trending badge if applicable
                                        if preset.trending.unwrap_or(false) {
                                            span { class: "trending-badge", "Popular" }
                                        }
                                        
                                        h4 { "{preset.name}" }
                                        p { "{preset.description}" }
                                        
                                        // Feature count badge
                                        span { class: "preset-features-count",
                                            "{preset.enabled_features.len()} features"
                                        }
                                        
                                        // Author if available
                                        if let Some(author) = &preset.author {
                                            div { class: "preset-author", "By {author}" }
                                        }
                                    }
                                }
                            }
                        }
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
                    
                    // Display features by category
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

// Helper function to render features by category
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
    
    // Group by category
    let mut categories: std::collections::BTreeMap<String, Vec<ModComponent>> = std::collections::BTreeMap::new();
    for component in filtered_components {
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
                    
                    // Calculate how many components are enabled
                    let enabled_count = enabled_features.read().iter()
                        .filter(|id| components.iter().any(|comp| &comp.id == *id))
                        .count();
                    
                    let are_all_enabled = enabled_count == components.len();
                    
                    rsx! {
                        div { class: "feature-category",
                            // Category header
                            div { class: "category-header",
                                div { class: "category-title-section",
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
                                    
                                    h3 { class: "category-name", "{category_name}" }
                                    span { class: "category-count", "{enabled_count}/{components.len()}" }
                                }
                                
                                // Toggle all button
                                {
                                    let components_clone = components.clone();
                                    let _category_name_clone = category_name.clone();
                                    let mut enabled_features = enabled_features.clone();
                                    
                                    rsx! {
                                        button {
                                            class: if are_all_enabled {
                                                "category-toggle-all disabled"
                                            } else {
                                                "category-toggle-all"
                                            },
                                            onclick: move |_| {
                                                // Toggle all in category
                                                enabled_features.with_mut(|features| {
                                                    if are_all_enabled {
                                                        // Disable all
                                                        for comp in &components_clone {
                                                            features.retain(|id| id != &comp.id);
                                                        }
                                                    } else {
                                                        // Enable all
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
                                {
                                    let category_key_clone = category_key.clone();
                                    
                                    rsx! {
                                        div { 
                                            class: if is_expanded {
                                                "category-toggle expanded"
                                            } else {
                                                "category-toggle"
                                            },
                                            onclick: move |_| {
                                                expanded_categories.with_mut(|cats| {
                                                    if cats.contains(&category_key_clone) {
                                                        cats.retain(|c| c != &category_key_clone);
                                                    } else {
                                                        cats.push(category_key_clone.clone());
                                                    }
                                                });
                                            },
                                            "▼"
                                        }
                                    }
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
                                                    
                                                    // Incompatibilities display
                                                    if let Some(incompats) = &component.incompatibilities {
                                                        if !incompats.is_empty() {
                                                            div { class: "feature-incompatibilities",
                                                                "Conflicts with: ", 
                                                                span { class: "incompatibility-list", 
                                                                    {incompats.join(", ")}
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
