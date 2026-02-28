use crate::{
    components::{
        separator::Separator,
        sidebar::{
            Sidebar, SidebarCollapsible, SidebarContent, SidebarFooter, SidebarGroup,
            SidebarGroupContent, SidebarGroupLabel, SidebarHeader, SidebarInset, SidebarMenu,
            SidebarMenuButton, SidebarMenuButtonSize, SidebarMenuItem, SidebarProvider,
            SidebarRail, SidebarSide, SidebarTrigger, SidebarVariant,
        },
        toast::ToastProvider,
    },
    models::Projects,
    Route,
};
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon, IconShape};

/// The Navbar component that will be rendered on all pages of our app since every page is under the layout.
///
///
/// This layout component wraps the UI of [Route::Home] and [Route::Blog] in a common navbar. The contents of the Home and Blog
/// routes will be rendered under the outlet inside this component
#[component]
pub fn Navbar() -> Element {
    // let projects = use_project_persistence();
    let projects = use_context::<Signal<Projects>>();
    let keys = use_memo(move || projects().keys().cloned().collect::<Vec<String>>());

    rsx! {
        // document::Link { rel: "stylesheet", href: NAVBAR_CSS }

        // div { id: "navbar",
        //     Link { to: Route::Home {}, "Home" }
        //     Link { to: Route::Blog { id: 1 }, "Blog" }
        //     Link { to: Route::SensorPanel {}, "Sensor" }
        //     Link { to: Route::DeviceSensorView {}, "Sensor2" }
        // }
        SidebarProvider {
            Sidebar {
                class: "",
                side: SidebarSide::Left,
                variant: SidebarVariant::Sidebar,
                collapsible: SidebarCollapsible::Offcanvas,

                SidebarHeader {
                    SidebarMenu {
                        SidebarMenuItem {
                            SidebarMenuButton { size: SidebarMenuButtonSize::Lg,
                                span { class: "text-xl", "Data Viewer" }
                            }
                        }
                    }
                }
                SidebarContent {
                    SidebarGroup {
                        SidebarGroupLabel { "Home" }
                        SidebarGroupContent {
                            SidebarMenu {
                                SidebarLink {
                                    to: Route::Home {},
                                    icon: fa_solid_icons::FaHouse,
                                    "Home"
                                }
                            }
                        }
                    }

                    SidebarGroup {
                        SidebarGroupLabel { "Projects" }
                        SidebarGroupContent {
                            SidebarMenu {
                                NavProjects { keys }
                                SidebarLink {
                                    to: Route::ProjectsView {},
                                    icon: fa_solid_icons::FaCirclePlus,
                                    "Add Project"
                                }
                            }
                        }
                    }

                    SidebarGroup {
                        SidebarGroupLabel { "Setting" }
                        SidebarGroupContent {
                            SidebarMenu {
                                SidebarLink {
                                    to: Route::EndpointView {},
                                    icon: fa_solid_icons::FaLink,
                                    "Endpoints"
                                }
                            }
                        }
                    }

                    SidebarGroup {
                        SidebarGroupLabel { "Other" }
                        SidebarContent {
                            SidebarLink {
                                to: Route::Blog { id: 1 },
                                icon: fa_solid_icons::FaHouse,
                                "Blog"
                            }
                            SidebarLink {
                                to: Route::SensorPanel {},
                                icon: fa_solid_icons::FaHouse,
                                "Sensor"
                            }
                            SidebarLink {
                                to: Route::Storage {},
                                icon: fa_solid_icons::FaLink,
                                "Storage"
                            }
                            SidebarLink {
                                to: Route::Storage2 {},
                                icon: fa_solid_icons::FaLink,
                                "Storage2"
                            }
                        }
                    }
                }
                SidebarFooter {}
                SidebarRail {}
            }

            SidebarInset {
                header { class: "flex items-center gap-2 h-14 p-4",
                    div { class: "flex items-center gap-3",
                        SidebarTrigger {}
                        Separator { height: "1rem", horizontal: false }
                        span { class: "text-lg font-bold", "Title" }
                    }
                }
                div { class: "overflow-y-auto",
                    div { class: "container max-w-7xl w-full mx-auto px-4",
                        ToastProvider { Outlet::<Route> {} }
                    }
                }
            }
        
        }

        // The `Outlet` component is used to render the next component inside the layout. In this case, it will render either
        // the [`Home`] or [`Blog`] component depending on the current route.
    }
}

#[component]
pub fn SidebarLink<T, Target>(to: Target, icon: T, children: Element) -> Element
where
    T: IconShape + Clone + PartialEq + 'static,
    Target: Into<NavigationTarget> + 'static + PartialEq + Clone,
{
    rsx! {
        SidebarMenuItem {
            SidebarMenuButton {
                size: SidebarMenuButtonSize::Lg,
                r#as: move |attributes: Vec<Attribute>| rsx! {
                    Link { attributes, to: to.clone().into(),
                        Icon { icon: icon.clone() }
                        {children.clone()}
                    }
                },
            }
        }
    }
}

#[component]
pub fn NavProjects(keys: ReadSignal<Vec<String>>) -> Element {
    rsx! {
        for k in keys.iter() {
            SidebarLink {
                to: Route::DevicePage3 {
                    project_name: k.to_string(),
                },
                icon: fa_solid_icons::FaFolderOpen,
                "{k}"
            }
        }
    }
}
