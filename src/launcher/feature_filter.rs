use dioxus::prelude::*;

#[derive(PartialEq, Props, Clone)]
pub struct FeatureFilterProps {
    pub filter_text: Signal<String>,  // Fixed: Added generic type parameter
}

#[component]
pub fn FeatureFilter(mut props: FeatureFilterProps) -> Element {  // Added mut to props
    rsx! {
        div { class: "feature-filter-container",
            span { class: "feature-filter-icon", "🔍" }
            input {
                r#type: "text",
                class: "feature-filter-input",
                placeholder: "Search features...",
                value: "{props.filter_text.read()}",
                oninput: move |evt| props.filter_text.set(evt.value().clone())
            }
            
            // Clear button
            if !props.filter_text.read().is_empty() {
                button {
                    class: "clear-filter-button",
                    onclick: move |_| props.filter_text.set(String::new()),
                    "×"
                }
            }
        }
    }
}
