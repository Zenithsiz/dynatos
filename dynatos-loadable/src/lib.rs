//! Loadable values for [`dynatos`]

// Features
#![feature(try_trait_v2, lint_reasons, never_type)]

// Modules
mod loadable;

// Exports
pub use loadable::{IntoLoaded, Loadable};
