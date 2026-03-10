use crate::{Route, views::global::HeaderContext};
use dioxus::prelude::*;

/// The Home page component that will be rendered when the current route is `[Route::Home]`.
///
/// This page gives a short introduction to the Data Viewer application and guides first-time
/// visitors through the initial setup steps: creating an endpoint and then adding a project.
#[component]
pub fn Home() -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("Home");
    });
    rsx! {
        div { class: "container mx-auto p-8 space-y-8",
            h1 { class: "text-4xl font-bold", "Data Viewer" }
            p { class: "text-lg",
                "Welcome to Data Viewer – a web application for managing and viewing sensor data projects."
            }

            // Getting Started Section
            div { class: "space-y-4",
                h2 { class: "text-2xl font-semibold", "Getting Started" }
                p { "If you're visiting for the first time, please follow these steps:" }
                ol { class: "list-decimal list-inside space-y-2 pl-4",
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
            }

            // Completed Features Section
            div { class: "space-y-3 p-4 rounded-lg bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800",
                h2 { class: "text-2xl font-semibold text-green-900 dark:text-green-100",
                    "✓ Completed Features"
                }
                ul { class: "list-disc list-inside space-y-2 pl-4 text-green-800 dark:text-green-200",
                    li {
                        "Manually input Project Key to view project content, including all devices and latest sensor data"
                    }
                    li {
                        "Configure Device and Sensor settings: device information, additional attributes, and active monitoring settings"
                    }
                    li {
                        "View sensor historical data, supporting numerical and image sensor types with historical image search"
                    }
                }
            }

            // Upcoming Features Section
            div { class: "space-y-3 p-4 rounded-lg bg-amber-50 dark:bg-amber-950 border border-amber-200 dark:border-amber-800",
                h2 { class: "text-2xl font-semibold text-amber-900 dark:text-amber-100",
                    "⏳ Upcoming Features"
                }
                ul { class: "list-disc list-inside space-y-2 pl-4 text-amber-800 dark:text-amber-200",
                    li { "Overview of active monitoring status" }
                    li { "Add and delete Device and Sensor" }
                    li { "WebSocket connection for real-time sensor data updates" }
                }
            }

            // Security Notice Section
            div { class: "bg-blue-50 dark:bg-blue-950 border border-blue-200 dark:border-blue-800 rounded-lg p-4 space-y-2",
                h3 { class: "font-semibold text-blue-900 dark:text-blue-100", "🔒 Data Security" }
                p { class: "text-sm text-blue-800 dark:text-blue-200",
                    "All settings and data in Endpoints and Add Project are stored locally in your browser. Your data is secure and private."
                }
            }

            p { class: "text-gray-600 dark:text-gray-400",
                "Use the navigation links in the sidebar to move around the application."
            }
        }
    }
}
