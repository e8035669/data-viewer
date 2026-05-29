use std::time::Duration;

use crate::{
    components::{
        button::{Button, ButtonVariant},
        card::{Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle},
        dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle},
        input::Input,
        radio_group::{RadioGroup, RadioItem},
    },
    models::{
        AuthInfo, AuthInfos, EdgeAuthInfo, Endpoints, GeneralAuthInfo, Project, ProjectResp,
        Projects,
    },
    ui::page_header::PageHeader,
    views::global::HeaderContext,
    Route,
};
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::{
    label::Label,
    toast::{use_toast, ToastOptions},
};

#[css_module("/src/components/dialog/style.css")]
struct Styles;

// ──────────────────────────────────────────────
// AuthInfosPage: list / add / delete AuthInfo
// ──────────────────────────────────────────────

#[derive(Store, Default)]
struct NewAuthInfoState {
    is_open: bool,
    name: String,
    kind: String,
    // General fields
    endpoint_url: String,
    x_api_key: String,
    // Edge fields
    edge_url: String,
    digest: String,
}

#[store]
impl<Lens> Store<NewAuthInfoState, Lens> {
    fn open_dialog(&mut self) {
        self.name().clear();
        self.endpoint_url().clear();
        self.x_api_key().clear();
        self.edge_url().clear();
        self.digest().clear();
        self.kind().set("General".to_string());
        self.is_open().set(true);
    }
}

#[derive(Store)]
struct DeleteAuthInfoState {
    is_open: bool,
    target: String,
}

#[store]
impl<Lens> Store<DeleteAuthInfoState, Lens> {
    fn prompt_delete(&mut self, target: &str) {
        self.target().set(target.to_string());
        self.is_open().set(true);
    }
}

#[component]
pub fn AuthInfosPage() -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("Credentials");
    });

    let mut auth_infos = use_context::<Signal<AuthInfos>>();

    let mut new_state = use_store(|| NewAuthInfoState {
        kind: "General".to_string(),
        ..Default::default()
    });

    let delete_state = use_store(|| DeleteAuthInfoState {
        is_open: false,
        target: String::new(),
    });

    let on_new_submit = move |_| {
        let name = new_state.name().take();
        let kind = new_state.kind().take();
        let toast_api = use_toast();

        if name.is_empty() || auth_infos.read().contains_key(&name) {
            toast_api.error(
                "Add AuthInfo failed".to_string(),
                ToastOptions::new()
                    .description("Name is empty or already exists")
                    .duration(Duration::from_secs(5)),
            );
            new_state.is_open().set(false);
            return;
        }

        let auth_info = if kind == "General" {
            AuthInfo::General(GeneralAuthInfo {
                endpoint: new_state.endpoint_url().take(),
                x_api_key: new_state.x_api_key().take(),
            })
        } else {
            AuthInfo::Edge(EdgeAuthInfo {
                url: new_state.edge_url().take(),
                digest: new_state.digest().take(),
            })
        };

        auth_infos.write().insert(name.clone(), auth_info);
        toast_api.success(
            format!("Added AuthInfo '{name}'"),
            ToastOptions::new().duration(Duration::from_secs(5)),
        );
        new_state.is_open().set(false);
    };

    let new_dialog = rsx! {
        DialogRoot {
            open: *new_state.is_open().read(),
            on_open_change: move |v| new_state.is_open().set(v),
            DialogContent {
                button {
                    class: Styles::dx_dialog_close,
                    r#type: "button",
                    aria_label: "Close",
                    tabindex: if *new_state.is_open().read() { "0" } else { "-1" },
                    onclick: move |_| new_state.is_open().set(false),
                    "×"
                }
                DialogTitle { "New Auth Info" }
                DialogDescription {
                    div { class: "flex flex-col gap-4",
                        Label { html_for: "auth_name", "Name" }
                        Input {
                            id: "auth_name",
                            oninput: move |e: FormEvent| new_state.name().set(e.value()),
                        }

                        Label { html_for: "auth_kind", "Kind" }
                        RadioGroup {
                            id: "auth_kind",
                            value: "{new_state.kind()}",
                            on_value_change: move |v| new_state.kind().set(v),
                            RadioItem { index: 0usize, value: "General", "General" }
                            RadioItem { index: 1usize, value: "Edge", "Edge" }
                        }

                        if new_state.kind()().as_str() == "General" {
                            Label { html_for: "general_url", "Endpoint URL" }
                            Input {
                                id: "general_url",
                                placeholder: "https://example.com/api/v1",
                                oninput: move |e: FormEvent| new_state.endpoint_url().set(e.value()),
                            }
                            Label { html_for: "general_key", "X-API-Key" }
                            Input {
                                id: "general_key",
                                oninput: move |e: FormEvent| new_state.x_api_key().set(e.value()),
                            }
                        } else {
                            Label { html_for: "edge_url", "Auth URL" }
                            Input {
                                id: "edge_url",
                                placeholder: "https://example.com/edge/v1/auth?username=...",
                                oninput: move |e: FormEvent| new_state.edge_url().set(e.value()),
                            }
                            Label { html_for: "edge_digest", "Digest" }
                            Input {
                                id: "edge_digest",
                                oninput: move |e: FormEvent| new_state.digest().set(e.value()),
                            }
                        }

                        Button { r#type: "submit", onclick: on_new_submit, "Submit" }
                    }
                }
            }
        }
    };

    let on_delete_confirm = move |_| {
        let target = delete_state.target().take();
        auth_infos.write().remove(&target);
        delete_state.is_open().set(false);
    };

    let delete_dialog = rsx! {
        DialogRoot {
            open: *delete_state.is_open().read(),
            on_open_change: move |v| delete_state.is_open().set(v),
            DialogContent {
                DialogTitle { "Delete Confirm" }
                DialogDescription {
                    div { class: "flex flex-col gap-4",
                        "Delete auth info '{delete_state.target()}'?"
                        div { class: "flex justify-end gap-4",
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: move |_| delete_state.is_open().set(false),
                                "No"
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
        if !auth_infos.read().is_empty() {
            for (name , info) in auth_infos().iter() {
                div { key: "{name}",
                    AuthInfoCard { name, info: info.clone(), delete_state }
                }
            }
        } else {
            p { "No auth info saved. Add one to get started." }
        }
    };

    rsx! {
        div { class: "mb-4 p-4 rounded-md border bg-muted/50 text-sm text-muted-foreground",
            p { class: "font-medium text-foreground mb-1", "Import Projects from API" }
            p {
                "Save your API credentials here, then click an entry to fetch its project list and import projects into the app."
            }
            p { class: "mt-2 text-yellow-600 dark:text-yellow-400",
                "⚠ Web version may fail due to CORS restrictions. Use the Linux desktop version for reliable operation."
            }
        }
        PageHeader { title: "Credentials",
            Button { onclick: move |_| new_state.open_dialog(), "New" }
        }
        {new_dialog}
        {delete_dialog}
        div { class: "grid grid-cols-1 gap-4", {cards} }
    }
}

#[component]
fn AuthInfoCard(name: String, info: AuthInfo, delete_state: Store<DeleteAuthInfoState>) -> Element {
    let name_for_delete = name.clone();
    let name_for_nav = name.clone();

    let (kind, summary) = match &info {
        AuthInfo::General(g) => ("General".to_string(), g.endpoint.clone()),
        AuthInfo::Edge(e) => ("Edge".to_string(), e.url.clone()),
    };

    rsx! {
        Card {
            CardHeader {
                CardTitle {
                    Link {
                        to: Route::ProjectImportPage {
                            auth_info_name: name_for_nav.clone(),
                        },
                        {name.as_str()}
                    }
                }
                CardDescription {
                    p { "Type: {kind}" }
                    p { class: "truncate", "URL: {summary}" }
                }
                CardAction {
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| delete_state.prompt_delete(&name_for_delete),
                        Icon { icon: fa_solid_icons::FaTrash }
                    }
                }
            }
        }
    }
}

// ──────────────────────────────────────────────
// ProjectImportPage: fetch and import projects
// ──────────────────────────────────────────────

#[derive(Store, Default)]
struct ImportDialogState {
    is_open: bool,
    project_name: String,
    project_key: String,
    endpoint_key: String,
    available_keys: Vec<String>,
}

#[store]
impl<Lens> Store<ImportDialogState, Lens> {
    fn open_for_project(&mut self, project: &ProjectResp) {
        let keys: Vec<String> = project.project_keys.iter().map(|k| k.key.clone()).collect();
        self.project_name().set(project.name.clone());
        self.project_key()
            .set(keys.first().cloned().unwrap_or_default());
        self.endpoint_key().clear();
        self.available_keys().set(keys);
        self.is_open().set(true);
    }
}

#[component]
pub fn ProjectImportPage(auth_info_name: String) -> Element {
    use_effect({
        let title = auth_info_name.clone();
        move || {
            consume_context::<HeaderContext>().set_title(&format!("Import: {title}"));
        }
    });

    let auth_infos = use_context::<Signal<AuthInfos>>();
    let mut projects = use_context::<Signal<Projects>>();
    let endpoints = use_context::<Signal<Endpoints>>();

    let auth_info_name_clone = auth_info_name.clone();
    let fetched_projects = use_resource(move || {
        let infos = auth_infos();
        let name = auth_info_name_clone.clone();
        async move {
            let info = infos.get(&name).cloned();
            match info {
                Some(auth) => auth.get_projects().await.ok(),
                None => None,
            }
        }
    });

    let import_state = use_store(ImportDialogState::default);

    let on_import_submit = move |_| {
        let name = import_state.project_name().take();
        let project_key = import_state.project_key().take();
        let endpoint_key = import_state.endpoint_key().take();
        let toast_api = use_toast();

        if name.is_empty() || project_key.is_empty() || endpoint_key.is_empty() {
            toast_api.error(
                "Import failed".to_string(),
                ToastOptions::new()
                    .description("Please fill all fields")
                    .duration(Duration::from_secs(5)),
            );
            import_state.is_open().set(false);
            return;
        }

        if projects.read().contains_key(&name) {
            toast_api.error(
                "Import failed".to_string(),
                ToastOptions::new()
                    .description("Project name already exists")
                    .duration(Duration::from_secs(5)),
            );
            import_state.is_open().set(false);
            return;
        }

        projects.write().insert(
            name.clone(),
            Project {
                project_key,
                endpoint_key,
            },
        );
        toast_api.success(
            format!("Imported project '{name}'"),
            ToastOptions::new().duration(Duration::from_secs(5)),
        );
        import_state.is_open().set(false);
    };

    let endpoint_copy = endpoints();
    let endpoint_items = endpoint_copy.keys().enumerate().map(|(i, k)| {
        rsx! {
            RadioItem { index: i, value: "{k}", "{k}" }
        }
    });

    let available_keys_copy = import_state.available_keys()();
    let key_items = available_keys_copy.iter().enumerate().map(|(i, k)| {
        rsx! {
            RadioItem { index: i, value: "{k}", "{k}" }
        }
    });

    let import_dialog = rsx! {
        DialogRoot {
            open: *import_state.is_open().read(),
            on_open_change: move |v| import_state.is_open().set(v),
            DialogContent {
                button {
                    class: Styles::dx_dialog_close,
                    r#type: "button",
                    aria_label: "Close",
                    tabindex: if *import_state.is_open().read() { "0" } else { "-1" },
                    onclick: move |_| import_state.is_open().set(false),
                    "×"
                }
                DialogTitle { "Import Project" }
                DialogDescription {
                    div { class: "flex flex-col gap-4",
                        Label { html_for: "import_name", "Project Name" }
                        Input {
                            id: "import_name",
                            value: "{import_state.project_name()}",
                            oninput: move |e: FormEvent| import_state.project_name().set(e.value()),
                        }

                        Label { html_for: "import_key", "Project Key" }
                        if import_state.available_keys()().len() > 1 {
                            RadioGroup {
                                id: "import_key",
                                value: import_state.project_key()(),
                                on_value_change: move |v: String| import_state.project_key().set(v),
                                {key_items}
                            }
                        } else {
                            Input {
                                id: "import_key",
                                value: "{import_state.project_key()}",
                                oninput: move |e: FormEvent| import_state.project_key().set(e.value()),
                            }
                        }

                        Label { html_for: "import_endpoint", "Endpoint" }
                        div { id: "import_endpoint",
                            if endpoints().is_empty() {
                                p { "No endpoints available. Add one first." }
                            } else {
                                RadioGroup {
                                    value: import_state.endpoint_key()(),
                                    on_value_change: move |v: String| import_state.endpoint_key().set(v),
                                    {endpoint_items}
                                }
                            }
                        }

                        Button { r#type: "submit", onclick: on_import_submit, "Import" }
                    }
                }
            }
        }
    };

    let project_list = match fetched_projects() {
        Some(Some(projects_list)) => rsx! {
            div { class: "grid grid-cols-1 gap-4",
                for project in projects_list.iter() {
                    ProjectFetchedCard { project: project.clone(), import_state }
                }
            }
        },
        Some(None) => rsx! {
            p { "Failed to fetch projects or auth info not found." }
        },
        None => rsx! {
            p { "Loading..." }
        },
    };

    rsx! {
        h2 { class: "text-lg font-semibold mb-4", "Projects from '{auth_info_name}'" }
        {import_dialog}
        {project_list}
    }
}

#[component]
fn ProjectFetchedCard(project: ProjectResp, import_state: Store<ImportDialogState>) -> Element {
    let project_clone = project.clone();
    let keys_display: String = project
        .project_keys
        .iter()
        .map(|k| k.key.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "{project.name}" }
                CardDescription {
                    p { "ID: {project.id}" }
                    if !project.desc.is_empty() {
                        p { "{project.desc}" }
                    }
                }
                CardAction {
                    Button { onclick: move |_| import_state.open_for_project(&project_clone),
                        "Import"
                    }
                }
            }
            CardContent {
                p { class: "text-sm text-muted-foreground truncate", "Keys: {keys_display}" }
            }
        }
    }
}
