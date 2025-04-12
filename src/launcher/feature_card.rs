use dioxus::prelude::*;
use crate::universal::ModComponent;

#[derive(Props, Clone, PartialEq)]
pub struct FeatureCardProps {
    pub mod_component: ModComponent,
    pub is_enabled: bool,
    pub toggle_feature: EventHandler<String>
}

#[component]
pub fn FeatureCard(props: FeatureCardProps) -> Element {
    // Clone values needed for closures
    let feature_id = props.mod_component.id.clone();
    let is_enabled = props.is_enabled;
    
    rsx! {
        div { 
            class: if is_enabled {"feature-card feature-enabled"} else {"feature-card feature-disabled"},
            
            div { class: "feature-card-header",
                h3 { class: "feature-card-title", "{props.mod_component.name}" }
                
                label {
                    class: if is_enabled {"feature-toggle-button enabled"} else {"feature-toggle-button disabled"},
                    input {
                        r#type: "checkbox",
                        checked: is_enabled,
                        onchange: move |_| props.toggle_feature.call(feature_id.clone()),
                        style: "display: none;"
                    }
                    {if is_enabled {"Enabled"} else {"Disabled"}}
                }
            }
            
            // Description display
            if let Some(description) = &props.mod_component.description {
                div { class: "feature-card-description", "{description}" }
            }
            
            // Dependencies display with simplified approach
            if let Some(deps) = &props.mod_component.dependencies {
                if !deps.is_empty() {
                    div { class: "feature-dependencies",
                        span { "Required: " }
                        
                        // Simply join the dependencies with commas
                        span { class: "dependency-list", "{deps.join(\", \")}" }
                    }
                }
            }
            
            // Incompatibilities display 
            if let Some(incompats) = &props.mod_component.incompatibilities {
                if !incompats.is_empty() {
                    div { class: "feature-incompatibilities",
                        span { "Incompatible with: " }
                        
                        // Simply join the incompatibilities with commas
                        span { class: "incompatibility-list", "{incompats.join(\", \")}" }
                    }
                }
            }
            
            // Author information
            if !props.mod_component.authors.is_empty() {
                div { class: "feature-authors",
                    span { "By: " }
                    
                    // Join authors with commas
                    for (index, author) in props.mod_component.authors.iter().enumerate() {
                        {
                            let is_last = index == props.mod_component.authors.len() - 1;
                            rsx! {
                                a {
                                    href: "{author.link}",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    class: "author-link",
                                    "{author.name}{if !is_last { \", \" } else { \"\" }}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
