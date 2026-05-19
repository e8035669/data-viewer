use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct BreadcrumbItem {
    pub label: String,
    pub to: Option<NavigationTarget>,
}

impl BreadcrumbItem {
    pub fn link(label: impl Into<String>, to: impl Into<NavigationTarget>) -> Self {
        Self {
            label: label.into(),
            to: Some(to.into()),
        }
    }

    pub fn current(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            to: None,
        }
    }
}

#[component]
pub fn Breadcrumb(items: Vec<BreadcrumbItem>) -> Element {
    rsx! {
        nav { class: "flex flex-wrap items-center gap-1 text-sm text-muted-foreground mb-3",
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    span { class: "select-none opacity-40", "/" }
                }
                if let Some(to) = item.to.clone() {
                    Link {
                        to,
                        class: "hover:text-foreground transition-colors max-w-32 truncate",
                        {item.label.clone()}
                    }
                } else {
                    span {
                        class: "text-foreground font-medium max-w-48 truncate",
                        {item.label.clone()}
                    }
                }
            }
        }
    }
}
