use dioxus::prelude::*;

use crate::{
    components::button::Button,
    views::darkmode::{use_theme_context, Theme},
};

#[component]
pub fn PreferencePage() -> Element {
    let theme = use_theme_context();

    rsx! {
        div { class: "flex gap-2",
            Button {
                onclick: move |_| async move {
                    theme.set_theme(Theme::Auto).await;
                },
                "Auto"
            }
            Button {
                onclick: move |_| async move {
                    theme.set_theme(Theme::Light).await;
                },
                "Light"
            }
            Button {
                onclick: move |_| async move {
                    theme.set_theme(Theme::Dark).await;
                },
                "Dark"
            }
        }
    }
}
