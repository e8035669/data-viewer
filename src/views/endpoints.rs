use std::{collections::HashMap, time::Duration};

use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::{
    label::Label,
    toast::{use_toast, ToastOptions},
};
use crate::models::{Endpoint, EndpointTrait, Endpoints, GeneralEndpoint, EdgeEndpoint};
use crate::persistence::{use_count_persistent, use_endpoints_persistent};

use crate::components::{
    button::{Button, ButtonVariant},
    card::{Card, CardAction, CardDescription, CardHeader, CardTitle},
    dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle},
    input::Input,
    radio_group::{RadioGroup, RadioItem},
};



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
                    class: "dialog-close",
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
                            placeholder: "https://example.com/api",
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
                        div { class: "flex flex-row-reverse gap-4",
                            Button {
                                variant: ButtonVariant::Destructive,
                                onclick: on_delete_confirm,
                                "Yes"
                            }
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: move |_| delete_info.is_open().set(false),
                                "NO"
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
                div { class: "mb-4", key: "{name}",
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
        {cards}
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
