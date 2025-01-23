use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    rsx! {
        div {
            id: "hero",
            div { id: "links",
                a { href: "https://packwiz.toyvo.dev", "Minecraft Modpack" }
            }
        }
    }
}
