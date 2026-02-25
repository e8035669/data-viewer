use std::{collections::HashMap, time::Duration};

use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::toast::{use_toast, ToastOptions};
use dioxus_sdk_storage::{use_synced_storage, LocalStorage};
use serde::{Deserialize, Serialize};

use crate::{
    components::{
        button::{Button, ButtonVariant},
        card::{Card, CardAction, CardContent, CardHeader, CardTitle},
        dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle},
        input::Input,
        label::Label,
        select::{
            Select, SelectGroup, SelectItemIndicator, SelectList, SelectOption, SelectTrigger,
            SelectValue,
        },
    },
    views::endpoints::Endpoints,
};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct Project {
    pub project_key: String,
    pub endpoint_key: String,
}

#[derive(Store)]
pub struct AddProjectCtx {
    pub is_open: bool,
    pub name: String,
    pub project_key: String,
    pub endpoint_key: String,
}

#[store]
impl<Lens> Store<AddProjectCtx, Lens> {
    fn open_dialog(&mut self) {
        self.name().clear();
        self.project_key().clear();
        self.endpoint_key().clear();
        self.is_open().set(true);
    }
}

#[derive(Store)]
pub struct DeleteCtx {
    pub is_open: bool,
    pub target: String,
}

#[store]
impl<Lens> Store<DeleteCtx, Lens> {
    // This will automatically require `Writable` on the lens since it takes `&mut self`
    fn prompt_delete(&mut self, target: &str) {
        self.target().set(target.to_string());
        self.is_open().set(true);
    }
}

#[component]
pub fn ProjectsView() -> Element {
    let mut projects = use_context::<Signal<Projects>>();
    // let mut projects = use_project_persistence();
    let endpoints = use_context::<Signal<Endpoints>>();
    // let endpoints = use_endpoints_persistent();

    let mut new_info = use_store(|| AddProjectCtx {
        is_open: false,
        name: String::new(),
        project_key: String::new(),
        endpoint_key: String::new(),
    });

    let delete_ctx = use_store(|| DeleteCtx {
        is_open: false,
        target: String::new(),
    });

    let on_new_submit = move |_| {
        let new_name = new_info.name().take();
        let project_key = new_info.project_key().take();
        let endpoint_key = new_info.endpoint_key().take();
        let toast_api = use_toast();

        if !new_name.is_empty() && !projects.contains_key(&new_name) {
            let new_project = Project {
                project_key,
                endpoint_key,
            };
            projects.write().insert(new_name.clone(), new_project);
            toast_api.success(
                format!("Add project '{new_name}' success"),
                ToastOptions::new().duration(Duration::from_secs(5)),
            );
        } else {
            toast_api.error(
                format!("Add project Failed"),
                ToastOptions::new()
                    .description("name is already exist or empty")
                    .duration(Duration::from_secs(5)),
            );
        }
        new_info.is_open().set(false);
    };

    let endpoint_copy = endpoints();
    let endpoint_menus = endpoint_copy.keys().enumerate().map(|(i, k)| {
        rsx! {
            SelectOption::<String> { index: i, value: "{k}",
                "{k}"
                SelectItemIndicator {}
            }
        }
    });

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
                DialogTitle { "Add Project" }
                DialogDescription {
                    div { class: "flex flex-col gap-4",
                        Label { html_for: "project_name", "Name" }
                        Input {
                            id: "project_name",
                            oninput: move |e: FormEvent| new_info.name().set(e.value()),
                        }

                        Label { html_for: "project_key", "Project Key" }
                        Input {
                            id: "project_key",
                            placeholder: "PK123456789",
                            oninput: move |e: FormEvent| new_info.project_key().set(e.value()),
                        }

                        Label { html_for: "endpoint_key", "Endpoint Key" }
                        Select::<String> {
                            id: "endpoint_key",
                            placeholder: "Select an endpoint...",
                            on_value_change: move |v: Option<String>| new_info.endpoint_key().set(v.unwrap_or_default()),
                            SelectTrigger { class: "w-48", aria_label: "Select Trigger", SelectValue {} }

                            SelectList {
                                SelectGroup { {endpoint_menus} }
                            }
                        }

                        Button { r#type: "submit", onclick: on_new_submit, "Submit" }
                    }
                }
            }
        }
    };

    let on_delete_confirm = move |_| {
        let target = delete_ctx.target().take();
        projects.remove(&target);
        delete_ctx.is_open().set(false);
    };

    let delete_dialog = rsx! {
        DialogRoot {
            open: *delete_ctx.is_open().read(),
            on_open_change: move |v| delete_ctx.is_open().set(v),
            DialogContent {
                DialogTitle { "Delete Confirm" }
                DialogDescription {
                    div { class: "flex flex-col gap-4",
                        "Delete endpoint {delete_ctx.target()}"
                        div { class: "flex flex-row-reverse gap-4",
                            Button {
                                variant: ButtonVariant::Destructive,
                                onclick: on_delete_confirm,
                                "Yes"
                            }
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: move |_| delete_ctx.is_open().set(false),
                                "NO"
                            }
                        }
                    }
                }
            }
        }
    };

    let cards = rsx! {
        if !projects.is_empty() {
            for (name , project) in projects().iter() {
                ProjectCard { name, project: project.clone(), delete_ctx }
            }
        } else {
            p { "No project, Add new one" }
        }
    };

    let on_new_click = move |_| {
        new_info.open_dialog();
    };

    rsx! {
        Button { onclick: on_new_click, "Add" }
        {new_dialog}
        {delete_dialog}
        {cards}
    }
}

#[component]
pub fn ProjectCard(name: String, project: Project, delete_ctx: Store<DeleteCtx>) -> Element {
    let name_clone = name.clone();
    let prompt_delete = move |_| {
        delete_ctx.prompt_delete(&name_clone);
    };
    rsx! {
        Card {
            CardHeader {
                CardTitle { {name} }
                CardAction {
                    Button { variant: ButtonVariant::Ghost, onclick: prompt_delete,
                        Icon { icon: fa_solid_icons::FaTrash }
                    }
                }
            }

            CardContent {
                p { "Project key: {project.project_key}" }
                p { "Endpoint: {project.endpoint_key}" }
            }
        }
    }
}

pub type Projects = HashMap<String, Project>;
pub fn use_project_persistence() -> Signal<Projects> {
    use_synced_storage::<LocalStorage, _>("projects".to_string(), || Projects::new())
}
