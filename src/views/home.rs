use crate::Route;
use dioxus::prelude::*;

/// The Home page component that will be rendered when the current route is `[Route::Home]`.
///
/// This page gives a short introduction to the Data Viewer application and guides first-time
/// visitors through the initial setup steps: creating an endpoint and then adding a project.
#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "container mx-auto p-8 space-y-6",
            h1 { class: "text-4xl font-bold", "Data Viewer" }
            p {
                "Welcome to Data Viewer – a web application for managing and viewing sensor data projects."
            }
            p { "If you're visiting for the first time, please follow these steps:" }
            ol { class: "list-decimal list-inside space-y-2",
                li {
                    "First, go to the "
                    Link { to: Route::EndpointView {}, "Endpoints" }
                    " page and add a new endpoint."
                }
                li {
                    "Then visit the "
                    Link { to: Route::ProjectsView {}, "Add Project" }
                    " page to create your first project."
                }
            }
            p { "Use the navigation links in the sidebar to move around the application." }
        }
    }
}
