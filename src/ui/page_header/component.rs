use dioxus::prelude::*;

#[component]
pub fn PageHeader(title: String, children: Element) -> Element {
    rsx! {
        div { class: "flex flex-col sm:flex-row sm:items-center justify-between gap-2 mb-4",
            h1 { class: "text-2xl font-bold truncate sm:mr-4", {title} }
            div { class: "flex items-center gap-2 flex-wrap", {children} }
        }
    }
}
