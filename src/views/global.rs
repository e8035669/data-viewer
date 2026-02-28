use dioxus::prelude::*;

use crate::persistence::{use_endpoints_persistent, use_project_persistence};



#[component]
pub fn Providers(children: Element) -> Element {
    let endpoints = use_endpoints_persistent();
    use_context_provider(|| endpoints);
    let projects = use_project_persistence();
    use_context_provider(|| projects);
    
    children
}