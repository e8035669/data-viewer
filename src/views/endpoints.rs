use std::{collections::HashMap, time::Duration};

use crate::persistence::{use_count_persistent, use_endpoints_persistent};
use crate::{
    components::toolbar::{Toolbar, ToolbarButton, ToolbarGroup, ToolbarSeparator},
    models::{EdgeEndpoint, Endpoint, EndpointTrait, Endpoints, GeneralEndpoint},
    views::global::HeaderContext,
};
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::{
    label::Label,
    toast::{use_toast, ToastOptions},
};

use crate::components::{
    button::{Button, ButtonVariant},
    card::{Card, CardAction, CardDescription, CardHeader, CardTitle},
    dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle},
    input::Input,
    radio_group::{RadioGroup, RadioItem},
};

#[css_module("/src/components/dialog/style.css")]
struct Styles;

#[derive(Store, Default)]
struct NewEndpointInfo {
    name: String,
    endpoint_url: String,
    kind: String,
    is_open: bool,
}

#[store]
impl<Lens> Store<NewEndpointInfo, Lens> {
    fn open_dialog(&mut self) {
        self.name().clear();
        self.endpoint_url().clear();
        self.is_open().set(true);
    }
}

#[derive(Store)]
pub struct DeleteInfo {
    pub is_open: bool,
    pub target: String,
}

#[store]
impl<Lens> Store<DeleteInfo, Lens> {
    // This will automatically require `Writable` on the lens since it takes `&mut self`
    fn prompt_delete(&mut self, target: &str) {
        self.target().set(target.to_string());
        self.is_open().set(true);
    }
}

#[component]
pub fn EndpointView() -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("Endpoints");
    });
    let mut endpoints = use_context::<Signal<Endpoints>>();
    // let mut endpoints = use_endpoints_persistent();
    let mut new_info = use_store(|| NewEndpointInfo {
        name: String::new(),
        endpoint_url: String::new(),
        kind: "General".to_string(),
        is_open: false,
    });

    let delete_info = use_store(|| DeleteInfo {
        is_open: false,
        target: String::new(),
    });

    let on_new_submit = move |_| {
        let new_name = new_info.name().take();
        let endpoint_url = new_info.endpoint_url().take();
        let kind = new_info.kind().take();
        let toast_api = use_toast();

        if !new_name.is_empty() && !endpoints.contains_key(&new_name) {
            let new_endpoint = if kind == "General" {
                Endpoint::General(GeneralEndpoint {
                    base_url: endpoint_url,
                })
            } else {
                Endpoint::Edge(EdgeEndpoint {
                    base_url: endpoint_url,
                })
            };

            endpoints.write().insert(new_name.clone(), new_endpoint);

            toast_api.success(
                format!("Add endpoint '{new_name}' success"),
                ToastOptions::new().duration(Duration::from_secs(5)),
            );
        } else {
            toast_api.error(
                format!("Add endpoint Failed"),
                ToastOptions::new()
                    .description("name is already exist or empty")
                    .duration(Duration::from_secs(5)),
            );
        }
        new_info.is_open().set(false);
    };

    let new_dialog = rsx! {
        DialogRoot {
            open: *new_info.is_open().read(),
            on_open_change: move |v| new_info.is_open().set(v),
            DialogContent {
                button {
                    class: Styles::dx_dialog_close,
                    r#type: "button",
                    aria_label: "Close",
                    tabindex: if *new_info.is_open().read() { "0" } else { "-1" },
                    onclick: move |_| new_info.is_open().set(false),
                    "×"
                }
                DialogTitle { "New Endpoint" }
                DialogDescription {
                    div { class: "flex flex-col gap-4",
                        Label { html_for: "endpoint_name", "Name" }
                        Input {
                            id: "endpoint_name",
                            oninput: move |e: FormEvent| new_info.name().set(e.value()),
                        }

                        Label { html_for: "endpoint_url", "Endpoint URL" }
                        Input {
                            id: "endpoint_url",
                            placeholder: "https://example.com/api/v1",
                            oninput: move |e: FormEvent| new_info.endpoint_url().set(e.value()),
                        }

                        Label { html_for: "kind", "Kind" }
                        RadioGroup {
                            id: "kind",
                            value: "{new_info.kind()}",
                            on_value_change: move |v| new_info.kind().set(v),
                            RadioItem { index: 0usize, value: "General", "General" }
                            RadioItem { index: 1usize, value: "Edge", "Edge" }
                        }

                        Button { r#type: "submit", onclick: on_new_submit, "Submit" }
                    }
                }
            }
        }
    };

    let on_delete_confirm = move |_| {
        let target = delete_info.target().take();
        endpoints.remove(&target);
        delete_info.is_open().set(false);
    };

    let delete_dialog = rsx! {
        DialogRoot {
            open: *delete_info.is_open().read(),
            on_open_change: move |v| delete_info.is_open().set(v),
            DialogContent {
                DialogTitle { "Delete Confirm" }
                DialogDescription {
                    div { class: "flex flex-col gap-4",
                        "Delete endpoint {delete_info.target()}"
                        div { class: "flex justify-end gap-4",
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: move |_| delete_info.is_open().set(false),
                                "NO"
                            }
                            Button {
                                variant: ButtonVariant::Destructive,
                                onclick: on_delete_confirm,
                                "Yes"
                            }
                        }
                    }
                }
            }
        }
    };

    let cards = rsx! {
        if endpoints.len() > 0 {
            for (name , endpoint) in endpoints().iter() {
                div { key: "{name}",
                    EndpointCard {
                        name,
                        endpoint: endpoint.clone(),
                        delete_info,
                    }
                }
            }
        } else {
            p { "No Endpoint, Add one." }
        }

    };

    let on_new_click = move |_| {
        new_info.open_dialog();
    };

    rsx! {
        Button { class: "mb-4", onclick: on_new_click, "New" }
        {new_dialog}
        {delete_dialog}
        div { class: "grid grid-cols-1 gap-4", {cards} }
    }
}

#[component]
pub fn EndpointCard(name: String, endpoint: Endpoint, delete_info: Store<DeleteInfo>) -> Element {
    let name_clone = name.clone();
    let prompt_delete = move |_| {
        delete_info.prompt_delete(&name_clone);
    };
    rsx! {
        Card {
            CardHeader {
                CardTitle { {name.as_str()} }
                CardDescription {
                    p { "Base URL: {endpoint.baseurl()}" }
                    p { "Type: {endpoint.kind()}" }
                }
                CardAction {
                    Button { variant: ButtonVariant::Ghost, onclick: prompt_delete,
                        Icon { icon: fa_solid_icons::FaTrash }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Storage() -> Element {
    let mut is_bold = use_signal(|| false);
    let mut is_italic = use_signal(|| false);
    let mut is_underline = use_signal(|| false);
    let mut text_align = use_signal(|| "left".to_string());

    rsx! {
        Toolbar { aria_label: "Text formatting", horizontal: false,
            ToolbarGroup {
                ToggleToolbarButton {
                    index: 0usize,
                    is_on: is_bold(),
                    on_click: move |_| is_bold.toggle(),
                    "Bold"
                }
                ToggleToolbarButton {
                    index: 1usize,
                    is_on: is_italic(),
                    on_click: move |_| is_italic.toggle(),
                    "Italic"
                }
                ToggleToolbarButton {
                    index: 2usize,
                    is_on: is_underline(),
                    on_click: move |_| is_underline.toggle(),
                    "Underline"
                }
            }
            ToolbarSeparator { horizontal: false }
            ToolbarGroup {
                ToggleToolbarButton {
                    index: 3usize,
                    is_on: text_align() == "left",
                    on_click: move |_| text_align.set("left".to_string()),
                    "Align Left"
                }
                ToggleToolbarButton {
                    index: 4usize,
                    is_on: text_align() == "center",
                    on_click: move |_| text_align.set("center".to_string()),
                    "Align Center"
                }
                ToggleToolbarButton {
                    index: 5usize,
                    is_on: text_align() == "right",
                    on_click: move |_| text_align.set("right".to_string()),
                    "Align Right"
                }
            }
        }
        p {
            max_width: "30rem",
            text_align: "{text_align}",
            font_weight: if is_bold() { "bold" } else { "normal" },
            font_style: if is_italic() { "italic" } else { "normal" },
            text_decoration: if is_underline() { "underline" } else { "none" },
            "This is a sample text that will be formatted according to the toolbar buttons you click. Try clicking the buttons above to see how the text formatting changes."
        }
    }
}

// persistence helpers are provided by `crate::persistence`

#[component]
pub fn Storage2() -> Element {
    let mut num = use_count_persistent();
    rsx! {
        div {
            button {
                onclick: move |_| {
                    *num.write() += 1;
                },
                "Increment"
            }
            div { "{*num.read()}" }
        }
    }
}

#[component]
fn ToggleToolbarButton(
    index: usize,
    is_on: bool,
    on_click: Callback<()>,
    children: Element,
) -> Element {
    rsx! {
        ToolbarButton {
            index,
            on_click,
            "data-state": if is_on { "on" } else { "off" },
            background: if is_on { "var(--light, var(--primary-color-5)) var(--dark, var(--primary-color-6))" } else { "" },
            color: if is_on { "var(--secondary-color-1)" } else { "" },
            {children}
        }
    }
}
