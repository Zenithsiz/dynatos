//! Loadable values for `dynatos`

// Features
#![feature(try_trait_v2, lint_reasons, never_type, extend_one)]

// Modules
pub mod loadable;

// Exports
pub use self::loadable::{IntoLoaded, IteratorLoadableExt, Loadable};
