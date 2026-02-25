use dioxus::prelude::*;

const HEADER_SVG: Asset = asset!("/assets/header.svg");

#[component]
pub fn Hero() -> Element {
    rsx! {
        // We can create elements inside the rsx macro with the element name followed by a block of attributes and children.
        div {
            // Attributes should be defined in the element before any children
            class: "flex flex-col justify-center items-center",
            id: "hero",
            // After all attributes are defined, we can define child elements and components
            img { class: "max-w-7xl", src: HEADER_SVG, id: "header" }
            div { class: "flex flex-col max-w-sm text-2xl", id: "links",
                // The RSX macro also supports text nodes surrounded by quotes
                HeroLink { href: "https://dioxuslabs.com/learn/0.7/", "📚 Learn Dioxus" }
                HeroLink { href: "https://dioxuslabs.com/awesome", "🚀 Awesome Dioxus" }
                HeroLink { href: "https://github.com/dioxus-community/", "📡 Community Libraries" }
                HeroLink { href: "https://github.com/DioxusLabs/sdk", "⚙️ Dioxus Development Kit" }
                HeroLink { href: "https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus",
                    "💫 VSCode Extension"
                }
                HeroLink { href: "https://discord.gg/XgGxMSkvUM", "👋 Community Discord" }
            }
        }
    }
}

#[component]
pub fn HeroLink(href: &'static str, children: Element) -> Element {
    rsx! {
        a {
            class: "border rounded-md mt-2.5 p-2.5 ml-2.5 hover:bg-gray-700",
            href,
            {children}
        }
    }
}
