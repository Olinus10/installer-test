use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct FeatureFilterProps {
    pub filter_text: Signal<String>,
}

#[component]
pub fn FeatureFilter(props: FeatureFilterProps) -> Element {
    rsx! {
        div { class: "feature-filter-container",
            span { class: "feature-filter-icon", "🔍" }
            input {
                class: "feature-filter",
                r#type: "text",
                placeholder: "Search for mods...",
                value: "{props.filter_text()}",
                oninput: move |evt| {
                    props.filter_text.set(evt.value().clone());
                }
            }
            
            // Clear button, only shown when there's text
            if !props.filter_text().is_empty() {
                button {
                    class: "feature-filter-clear",
                    onclick: move |_| props.filter_text.set(String::new()),
                    "×"
                }
            }
        }
    }
}
