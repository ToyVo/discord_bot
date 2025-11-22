pub mod app;
/// Define a components module that contains all shared components for our app.
pub mod components;
#[cfg(feature = "server")]
pub mod discord;
pub mod error;
pub mod state;
/// Define a views module that contains the UI for all Layouts and Routes for our app.
pub mod views;

rust_i18n::i18n!();
