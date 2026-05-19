use dioxus::prelude::*;

#[component]
pub fn PageHeader(title: String, children: Element) -> Element {
    rsx! {
        div { class: "flex items-center justify-between mb-4",
            h1 { class: "text-2xl font-bold truncate mr-4", {title} }
            div { class: "flex items-center gap-2 shrink-0", {children} }
        }
    }
}
