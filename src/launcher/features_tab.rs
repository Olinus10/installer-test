use dioxus::prelude::*;
use crate::universal::{ModComponent, UniversalManifest};
use crate::preset::{Preset, find_preset_by_id};
use crate::launcher::integrated_features::IntegratedFeatures;

#[component]
pub fn FeaturesTab(
    universal_manifest: Option<UniversalManifest>,
    presets: Vec<Preset>,
    enabled_features: Signal<Vec<String>>,
    selected_preset: Signal<Option<String>>,
    filter_text: Signal<String>,
) -> Element {
    // Handle changing a preset
    let mut apply_preset = move |preset_id: String| {
        if let Some(preset) = find_preset_by_id(&presets, &preset_id) {
            // Update enabled features
            enabled_features.set(preset.enabled_features.clone());
            
            // Mark as selected
            selected_preset.set(Some(preset_id));
        }
    };
    
    // Handle toggling a feature
    let mut toggle_feature = move |feature_id: String| {
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
    
    rsx! {
        div { class: "features-tab",
            h2 { "Features & Presets" }
            p { "Choose a preset or customize individual features to match your preferences." }
            
            if let Some(manifest) = universal_manifest {
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
                
                // Render the integrated features component
                IntegratedFeatures {
                    presets: presets,
                    selected_preset: selected_preset,
                    apply_preset: EventHandler::new(move |preset_id: String| {
                        apply_preset(preset_id)
                    }),
                    mod_components: all_components,
                    enabled_features: enabled_features,
                    toggle_feature: EventHandler::new(move |feature_id: String| {
                        toggle_feature(feature_id)
                    }),
                    filter_text: Some(filter_text),
                }
            } else {
                div { class: "loading-container",
                    div { class: "loading-spinner" }
                    div { class: "loading-text", "Loading features..." }
                }
            }
        }
    }
}
