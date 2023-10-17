#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::OptiWayApp;
mod app_init;
pub use app_init::{ setup_custom_fonts, setup_custom_styles };
pub mod md_icons;
