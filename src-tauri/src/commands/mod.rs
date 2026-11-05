//! Tauri commands, grouped by feature. Each command validates input, calls `l8-core` and maps
//! errors to [`crate::error::CommandError`].

pub mod artwork;
pub mod cloud;
pub mod library;
pub mod saves;
pub mod settings;
pub mod system;
