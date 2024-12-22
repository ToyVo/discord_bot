use crate::app::Route;
use dioxus::prelude::*;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }

        div {
            id: "navbar",
            Link {
                to: Route::Home {},
                "Home"
            }
            Link {
                to: Route::Logs {},
                "Logs"
            }
            Link {
                to: Route::TermsOfService {},
                "Terms of Service"
            }
            Link {
                to: Route::PrivacyPolicy {},
                "Privacy Policy"
            }
        }

        Outlet::<Route> {}
    }
}
