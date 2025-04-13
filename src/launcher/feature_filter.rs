use dioxus::prelude::*;

pub struct FeatureFilterProps {
    pub filter_text: Signal<String>,
}

#[component]
pub fn FeatureFilter(props: FeatureFilterProps) -> Element {
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
