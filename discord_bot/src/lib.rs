pub mod app;
pub mod components;
#[cfg(feature = "server")]
pub mod discord;
pub mod error;
pub mod state;
pub mod views;

rust_i18n::i18n!();
