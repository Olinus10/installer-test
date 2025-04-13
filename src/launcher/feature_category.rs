use dioxus::prelude::*;
// Remove this import if it causes conflicts
// use crate::launcher::FeatureCard;
use crate::universal::ModComponent;
use crate::launcher::FeatureCard;

#[derive(PartialEq, Props, Clone)]
pub struct FeatureCategoryProps {
    pub category_name: String,
    pub mods: Vec,
    pub enabled_features: Signal<Vec>,
    pub toggle_feature: EventHandler,
}

#[component]
pub fn FeatureCategory(props: FeatureCategoryProps) -> Element {
    let mut expanded = use_signal(|| false);
    
    // Count enabled mods in this category
    let enabled_count = props.enabled_features.read().iter()
        .filter(|id| props.mods.iter().any(|m| &m.id == *id))
        .count();
    
    rsx! {
        div { class: "feature-category",
            // Category header
            div { 
                class: "category-header",
                onclick: move |_| expanded.set(!*expanded.read()),
                
                div { class: "category-title-section",
                    h3 { class: "category-name", "{props.category_name}" }
                    span { class: "category-count", "{enabled_count}/{props.mods.len()}" }
                }
                
                div { 
                    class: if *expanded.read() { "category-toggle expanded" } else { "category-toggle" },
                    "▼"
                }
            }
            
            // Category content (expandable)
            div { 
                class: if *expanded.read() { "category-content expanded" } else { "category-content" },
                
                div { class: "feature-cards-grid",
                    // Display mod cards in this category
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
