//! Shared UI components — error popup overlay and status bar.

mod error_popup;
mod status_bar;

pub use error_popup::render as render_error_popup;
pub use status_bar::render as render_status_bar;
