//! Loadable values for `dynatos`

// Features
#![feature(try_trait_v2, lint_reasons, never_type, extend_one)]

// Modules
mod lazy_loadable;
pub mod loadable;

// Exports
pub use self::{
	lazy_loadable::LazyLoadable,
	loadable::{IntoLoaded, IteratorLoadableExt, Loadable},
};
