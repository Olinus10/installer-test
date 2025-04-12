use dioxus::prelude::*;
use crate::universal::ModComponent;
use crate::launcher::FeatureCard;

#[derive(Props, Clone, PartialEq)]
pub struct FeatureCategoryProps {
    pub category_name: String,
    pub mods: Vec<ModComponent>,
    pub enabled_features: Signal<Vec<String>>,
    pub toggle_feature: EventHandler<String>,
}

#[component]
pub fn FeatureCategory(props: FeatureCategoryProps) -> Element {
    // State for expanded/collapsed
    let mut is_expanded = use_signal(|| true);
    
    // Calculate if category is fully enabled
    let is_category_fully_enabled = props.mods.iter()
        .all(|m| props.enabled_features.read().contains(&m.id));
    
    // Calculate count of enabled mods in this category
    let enabled_count = props.mods.iter()
        .filter(|m| props.enabled_features.read().contains(&m.id))
        .count();
        
    // Toggle all mods in this category
    let toggle_all = move |_| {
        if is_category_fully_enabled {
            // Disable all in category
            for m in &props.mods {
                props.enabled_features.with_mut(|features| {
                    features.retain(|id| id != &m.id);
                });
            }
        } else {
            // Enable all in category
            for m in &props.mods {
                props.enabled_features.with_mut(|features| {
                    if !features.contains(&m.id) {
                        features.push(m.id.clone());
                    }
                });
            }
        }
    };
    
    rsx! {
        div { class: "feature-category-section",
            // Category header with toggle functionality
            div { 
                class: "category-header",
                onclick: move |_| is_expanded.set(!is_expanded()),
                
                h3 { 
                    class: "category-name", 
                    "{props.category_name} " 
                    span { 
                        class: "installed-count", 
                        "{enabled_count}/{props.mods.len()}" 
                    }
                }
                
                // Expand/collapse indicator
                div {
                    class: if *is_expanded.read() {
                        "category-toggle-indicator expanded"
                    } else {
                        "category-toggle-indicator"
                    },
                    "▼"
                }
            }
            
            // Toggle all button
            button {
                class: if is_category_fully_enabled {
                    "category-toggle-button enabled"
                } else {
                    "category-toggle-button disabled"
                },
                onclick: toggle_all,
                
                if is_category_fully_enabled {
                    "Disable All"
                } else {
                    "Enable All"
                }
            }
            
            // Category content (collapsible)
            div { 
                class: if *is_expanded.read() {
                    "category-content expanded"
                } else {
                    "category-content"
                },
                
                if *is_expanded.read() {
                    rsx! {
                        div { class: "features-grid",
                            for mod_component in &props.mods {
                                {
                                    let mod_id = mod_component.id.clone();
                                    let is_enabled = props.enabled_features.read().contains(&mod_id);
                                    
                                    rsx! {
                                        FeatureCard {
                                            key: "{mod_id}",
                                            mod_component: mod_component.clone(),
                                            is_enabled: is_enabled,
                                            toggle_feature: props.toggle_feature.clone()
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
