use dioxus::prelude::*;

use crate::persistence::{use_endpoints_persistent, use_project_persistence};

#[derive(Clone, Copy)]
pub struct HeaderContext {
    pub title: Signal<String>,
}

impl HeaderContext {
    pub fn reset(&mut self) {
        self.title.set("Title".to_string());
    }

    pub fn set_title(&mut self, title: &str) {
        self.title.set(title.to_string());
    }
}

#[component]
pub fn Providers(children: Element) -> Element {
    let endpoints = use_endpoints_persistent();
    use_context_provider(|| endpoints);
    let projects = use_project_persistence();
    use_context_provider(|| projects);

    let title = use_signal(|| "Title".to_string());
    use_context_provider(|| HeaderContext { title });

    children
}
