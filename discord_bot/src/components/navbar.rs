use {crate::app::Route, dioxus::prelude::*};

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

/// The Navbar component that will be rendered on all pages of our app since every page is under the layout.
///
///
/// This layout component wraps the UI of [Route::Home] in a common navbar. The contents of the Home
/// routes will be rendered under the outlet inside this component
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

        // The `Outlet` component is used to render the next component inside the layout. In this case, it will render either
        // the [`Home`] component depending on the current route.
        Outlet::<Route> {}
    }
}
