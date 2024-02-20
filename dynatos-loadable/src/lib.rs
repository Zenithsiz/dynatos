//! Loadable values for [`dynatos`]

// Features
#![feature(try_trait_v2, lint_reasons, never_type)]

// Modules
mod lazy_loadable;
mod loadable;

// Exports
pub use self::{
	lazy_loadable::LazyLoadable,
	loadable::{IntoLoaded, Loadable},
};
