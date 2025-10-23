//! The views module contains the components for all Layouts and Routes for our app. Each layout and route in our [`Route`]
//! enum will render one of these components.
//!
//!
//! The [`Home`] components will be rendered when the current route is [`Route::Home`] respectively.
//!
//!
//! The [`Navbar`] component will be rendered on all pages of our app since every page is under the layout. The layout defines
//! a common wrapper around all child routes.

mod home;
pub use home::Home;

mod logs;
pub use logs::Logs;

mod privacy_policy;
pub use privacy_policy::PrivacyPolicy;

mod terms_of_service;
pub use terms_of_service::TermsOfService;
