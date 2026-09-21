//! Declarative tools defined as JSON manifests.
//!
//! A manifest ([`ToolManifest`]) describes *what* the model sees (name, description,
//! parameters) and *how* Cephi executes it ([`Action`]). The model never gets raw shell
//! access: parameters are validated ([`args`]), rendered through context-aware escaping
//! ([`template`]) and passed to PowerShell only as environment variables.

mod args;
mod builtin;
mod executor;
mod localized;
mod manifest;
mod registry;
pub mod select;
mod template;
mod trigger;

pub use executor::{accepts_fast_path, execute, preview, ExecContext, ToolDisplay, ToolOutput};
pub use localized::LocalizedText;
pub use manifest::Permission;
pub use registry::ToolRegistry;
