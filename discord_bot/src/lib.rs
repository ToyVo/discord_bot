pub mod app;
pub mod components;
#[cfg(feature = "server")]
pub mod discord;
pub mod error;
#[cfg(feature = "server")]
pub mod models;
#[cfg(feature = "server")]
pub mod queries;
#[cfg(feature = "server")]
pub mod schema;
pub mod state;
pub mod views;

rust_i18n::i18n!();
