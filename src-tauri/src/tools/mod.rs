//! Declarative tools defined as JSON manifests.
//!
//! A manifest ([`ToolManifest`]) describes *what* the model sees (name, description,
//! parameters) and *how* Orbit executes it ([`Action`]). The model never gets raw shell
//! access: parameters are validated ([`args`]), rendered through context-aware escaping
//! ([`template`]) and passed to PowerShell only as environment variables.

mod args;
mod builtin;
mod executor;
mod manifest;
mod registry;
mod template;
mod trigger;

pub use executor::{accepts_fast_path, execute, preview, ExecContext, ToolOutput};
pub use manifest::Permission;
pub use registry::ToolRegistry;
