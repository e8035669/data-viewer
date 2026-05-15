use crate::{Route, views::global::HeaderContext};
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons::FaArrowRight, Icon};

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
                        "First, use the "
                        Link {
                            to: Route::EndpointView {},
                            class: "inline-flex items-center gap-1 text-blue-600 dark:text-blue-400 font-medium underline underline-offset-2 hover:text-blue-800 dark:hover:text-blue-300 transition-colors",
                            "Endpoints"
                            Icon { icon: FaArrowRight, width: 12, height: 12 }
                        }
                        " page to add a new endpoint."
                    }
                    li {
                        "Visit the "
                        Link {
                            to: Route::ProjectsView {},
                            class: "inline-flex items-center gap-1 text-blue-600 dark:text-blue-400 font-medium underline underline-offset-2 hover:text-blue-800 dark:hover:text-blue-300 transition-colors",
                            "Projects"
                            Icon { icon: FaArrowRight, width: 12, height: 12 }
                        }
                        " page to create or import your first project."
                    }
                }
            }

            // Completed Features Section
            div { class: "space-y-3 p-4 rounded-lg bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800",
                h2 { class: "text-2xl font-semibold text-green-900 dark:text-green-100",
                    "✓ Completed Features"
                }
                ul { class: "list-disc list-inside space-y-2 pl-4 text-green-800 dark:text-green-200",
                    li { "Project management: add/delete projects, import projects from Auth Info" }
                    li { "View project content including all devices and latest sensor data" }
                    li {
                        "Configure Device and Sensor settings: device information, additional attributes, and active monitoring settings"
                    }
                    li {
                        "View sensor historical data, supporting numerical and image sensor types with historical image search"
                    }
                    li {
                        "Active monitoring overview: per-project device monitoring status with auto-refresh"
                    }
                    li { "Rule configuration for monitoring alerts and notifications" }
                    li { "ROI drawing tool: draw, edit, and manage Regions of Interest on images" }
                    li {
                        "Auth Info management: configure General and Edge authentication credentials for project import"
                    }
                }
            }

            // Upcoming Features Section
            div { class: "space-y-3 p-4 rounded-lg bg-amber-50 dark:bg-amber-950 border border-amber-200 dark:border-amber-800",
                h2 { class: "text-2xl font-semibold text-amber-900 dark:text-amber-100",
                    "⏳ Upcoming Features"
                }
                ul { class: "list-disc list-inside space-y-2 pl-4 text-amber-800 dark:text-amber-200",
                    li { "WebSocket connection for real-time sensor data updates" }
                    li { "Dashboard with data visualization and charts" }
                    li { "Batch device and sensor operations" }
                }
            }

            // Security Notice Section
            div { class: "bg-blue-50 dark:bg-blue-950 border border-blue-200 dark:border-blue-800 rounded-lg p-4 space-y-2",
                h3 { class: "font-semibold text-blue-900 dark:text-blue-100", "🔒 Data Security" }
                p { class: "text-sm text-blue-800 dark:text-blue-200",
                    "All settings including endpoints, projects, and auth info are stored locally in your browser. Your data is secure and private."
                }
            }

            p { class: "text-gray-600 dark:text-gray-400",
                "Use the navigation links in the sidebar to move around the application."
            }
        }
    }
}
