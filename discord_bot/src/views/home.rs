use dioxus::prelude::*;

/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn Home() -> Element {
    rsx! {
        // We can create elements inside the rsx macro with the element name followed by a block of attributes and children.
        div {
            // Attributes should be defined in the element before any children
            id: "hero",
            // After all attributes are defined, we can define child elements and components
            div { id: "links",
                // The RSX macro also supports text nodes surrounded by quotes
                a { href: "https://packwiz.toyvo.dev", "Minecraft Modpack" }
            }
        }
    }
}
