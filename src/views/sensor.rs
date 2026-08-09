use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use crate::components::button::{Button, ButtonVariant};
use crate::components::card::{
    Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use crate::components::dialog::{Dialog, DialogDescription, DialogTitle};
use crate::components::dropdown_menu::{
    DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
};
use crate::components::input::Input;
use crate::components::label::Label;
use crate::components::radio_group::{RadioGroup, RadioItem};
use crate::components::select::{Select, SelectGroup, SelectOption};
use crate::components::switch::Switch;
use crate::components::textarea::Textarea;
use crate::views::global::HeaderContext;
use anyhow::{anyhow, Error, Result};
use async_std::task::sleep;
use base64::prelude::*;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_free_icons::icons::fa_solid_icons;
use dioxus_free_icons::Icon;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use reqwest::{Client, Url};
use strum::IntoEnumIterator;
use time::format_description::well_known::Iso8601;
use time::macros::{datetime, format_description, offset};
use time::{Date, OffsetDateTime};

use crate::models::{
    Action, ActionType, ActiveDevice, ActiveInfo, ActiveNotify, ActiveNotifySetting, Attribute,
    Device, EditDevice, EditSensor, Endpoint, EndpointTrait, Endpoints, GetRawData, Project,
    Projects, RawData, Rule1, Sensor, SensorStoreExt, SensorType, SensorWithData,
};

#[css_module("/src/components/input/style.css")]
struct InputStyles;


#[component]
pub fn TestRule1() -> Element {
    use_effect(move || {
        consume_context::<HeaderContext>().set_title("TestRule1");
    });
    let projects = use_context::<Signal<Projects>>();
    let endpoints = use_context::<Signal<Endpoints>>();

    let mut selected_project_key = use_signal(|| projects().keys().next().cloned());

    let project = use_memo(move || {
        let projects = projects();
        selected_project_key()
            .map(|k| projects.get(&k))
            .unwrap_or_default()
            .cloned()
    });
    let endpoint = use_memo(move || {
        let endpoints = endpoints();
        project()
            .map(|p| endpoints.get(&p.endpoint_key).cloned())
            .unwrap_or_default()
    });

    let content = if project().is_some() && endpoint().is_some() {
        let project = project().unwrap();
        let endpoint = endpoint().unwrap();
        rsx! {
            Rule1View { project, endpoint }
        }
    } else {
        rsx! {}
    };

    let projects = projects();
    let select_options = projects.keys().enumerate().map(|(i, k)| {
        rsx! {
            SelectOption::<String> { index: i, value: k.to_string(), {k.to_string()} }
        }
    });

    rsx! {
        Select::<String> {
            default_value: selected_project_key(),
            on_value_change: move |v: Option<String>| selected_project_key.set(v),

            SelectGroup { {select_options} }
        }

        {content}
        div { class: "h-96" }
    }
}

#[component]
fn Rule1View(project: ReadSignal<Project>, endpoint: ReadSignal<Endpoint>) -> Element {
    let rules_res: Resource<Result<Vec<Rule1>>> = use_resource(move || async move {
        let client = reqwest::Client::new();
        let url = endpoint().all_expression();
        let ret = client
            .get(url)
            .header("CK", project().project_key.as_str())
            .send()
            .await?
            .json::<Vec<Rule1>>()
            .await?;
        Ok(ret)
    });

    let content = if let Some(response) = &*rules_res.read() {
        match response {
            Ok(rules) => {
                if rules.is_empty() {
                    rsx! {
                        div { class: "text-center py-12 px-4",
                            p { class: "text-slate-500 dark:text-slate-400", "No rules configured" }
                        }
                    }
                } else {
                    rsx! {
                        div { class: "grid grid-cols-1 gap-4",
                            for rule in rules {
                                RuleCard { rule: rule.clone() }
                            }
                        }
                    }
                }
            }
            Err(e) => rsx! {
                div { class: "bg-red-50 dark:bg-red-950 border border-red-300 dark:border-red-700 rounded p-4 text-red-700 dark:text-red-300",
                    p { class: "font-semibold", "Error loading rules" }
                    p { class: "text-sm mt-1", "{e}" }
                }
            },
        }
    } else {
        rsx! {
            div { class: "text-center py-12 px-4",
                p { class: "text-slate-500 dark:text-slate-400", "Loading rules..." }
            }
        }
    };

    rsx! {
        div { class: "w-full",
            div { class: "grid grid-cols-1 gap-4 mb-6",
                div { class: "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2",
                    h2 { class: "text-2xl sm:text-3xl font-bold", "Rules" }
                    p { class: "text-sm text-slate-500 dark:text-slate-400",
                        "Project: {project().project_key}"
                    }
                }
            }

            {content}
        }
    }
}

#[component]
fn RuleCard(rule: Rule1) -> Element {
    let enable_badge_style = if rule.enable == "true" {
        "bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200"
    } else {
        "bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200"
    };

    rsx! {
        Card {
            CardHeader {
                CardTitle { class: "text-base md:text-lg", "{rule.name}" }
                CardDescription { class: "mt-1 text-sm", "{rule.desc}" }
                CardAction {
                    div { class: "flex gap-2",
                        span { class: "px-2 py-1 rounded text-xs font-medium {enable_badge_style}",
                            "{rule.enable}"
                        }
                        span { class: "px-2 py-1 rounded text-xs font-medium bg-purple-100 dark:bg-purple-900 text-purple-800 dark:text-purple-200",
                            "{rule.mode:?}"
                        }
                    
                    }
                }
            }

            CardContent {
                // Sensor and Devices row (mobile-first: stacked, then 2-column on md)
                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                    div { class: "min-w-0",
                        p { class: "text-xs font-semibold uppercase tracking-wide mb-2 opacity-70",
                            "Sensor"
                        }
                        div { class: "p-2 rounded text-sm font-mono bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-700 break-all",
                            "{rule.sensor}"
                        }
                    }
                    div { class: "min-w-0",
                        p { class: "text-xs font-semibold uppercase tracking-wide mb-2 opacity-70",
                            "Devices"
                        }
                        div { class: "flex flex-wrap gap-2",
                            for device in &rule.devices {
                                span { class: "inline-flex items-center px-2 py-1 rounded text-xs bg-slate-200 dark:bg-slate-700 text-slate-800 dark:text-slate-200",
                                    "{device}"
                                }
                            }
                        }
                    }
                }

                // Expression section
                div { class: "border-slate-200 dark:border-slate-700 pt-4",
                    p { class: "text-xs font-semibold uppercase tracking-wide mb-2 opacity-70",
                        "Expression"
                    }
                    div { class: "p-3 rounded text-xs font-mono bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-700 overflow-x-auto",
                        "{rule.expression}"
                    }
                }

                // Actions section
                if !rule.actions.is_empty() {
                    div { class: "border-slate-200 dark:border-slate-700 pt-4",
                        p { class: "text-xs font-semibold uppercase tracking-wide mb-3 opacity-70",
                            "Actions"
                        }
                        div { class: "grid grid-cols-1 gap-3",
                            for action in &rule.actions {
                                ActionItemDisplay { action: action.clone() }
                            }
                        }
                    }
                }

                // Rule ID in footer if present
                if let Some(id) = rule.id {
                    div { class: "text-xs opacity-50 border-t border-slate-200 dark:border-slate-700 pt-3",
                        "Rule ID: {id}"
                    }
                }
            }
        }
    }
}

#[component]
fn ActionItemDisplay(action: Action) -> Element {
    let container_style = match action.action_type {
        ActionType::EventAction => {
            "border-blue-300 dark:border-blue-700 bg-blue-50 dark:bg-blue-950"
        }
        ActionType::RecoverAction => {
            "border-green-300 dark:border-green-700 bg-green-50 dark:bg-green-950"
        }
    };

    let action_type_text_color = match action.action_type {
        ActionType::EventAction => "text-blue-700 dark:text-blue-300",
        ActionType::RecoverAction => "text-green-700 dark:text-green-300",
    };

    rsx! {
        div { class: "border rounded p-3 {container_style}",
            div { class: "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 mb-3",
                p { class: "font-semibold text-sm {action_type_text_color}", "{action.name}" }
                span { class: "text-xs px-2 py-1 rounded w-fit bg-slate-200 dark:bg-slate-700 text-slate-700 dark:text-slate-300",
                    "{action.action_type:?}"
                }
            }

            if let Some(email) = &action.email_event {
                div { class: "text-xs space-y-1.5 {action_type_text_color}",
                    div { class: "flex flex-col gap-1",
                        p { class: "font-semibold", "📧 Email" }
                        p { class: "break-all", "{email.email}" }
                    }
                    div { class: "flex flex-col gap-1",
                        p { class: "font-semibold", "Subject" }
                        p { class: "break-words", "{email.subject}" }
                    }
                    div { class: "flex flex-col gap-1",
                        p { class: "font-semibold", "Content" }
                        p { class: "break-words", "{email.content}" }
                    }
                }
            }

            if let Some(device) = &action.device_event {
                div { class: "text-xs space-y-1.5 {action_type_text_color}",
                    div { class: "flex flex-col gap-1",
                        p { class: "font-semibold", "Device ID" }
                        p { class: "font-mono break-all", "{device.device_id}" }
                    }
                    div { class: "flex flex-col gap-1",
                        p { class: "font-semibold", "Sensor ID" }
                        p { class: "font-mono break-all", "{device.sensor_id}" }
                    }
                    div { class: "flex flex-col gap-1",
                        p { class: "font-semibold", "Value" }
                        p { class: "font-mono break-all", "{device.value}" }
                    }
                }
            }
        }
    }
}
