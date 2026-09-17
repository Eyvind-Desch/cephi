//! API keys and other secrets.
//!
//! Secrets never live in JSON files (so they cannot end up in git or backups by accident) and
//! are never sent to the frontend; the UI only learns whether a secret is set.
//!
//! - [`store`]: storage in the Windows Credential Manager
//! - [`catalog`]: known services (grouping, labels, help) for the keys view

mod catalog;
mod store;

pub use catalog::{overview, SecretGroupInfo};
pub use store::{delete, get, is_set, is_valid_name, set};

#[cfg(test)]
pub(crate) use store::tests as test_support;
