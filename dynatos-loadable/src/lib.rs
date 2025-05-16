//! Loadable values for `dynatos`

// Features
#![feature(try_trait_v2, never_type, extend_one, unboxed_closures, negative_impls)]

// Modules
pub mod loadable;
pub mod loadable_signal;

// Exports
pub use self::{
	loadable::{IntoLoaded, IteratorLoadableExt, Loadable},
	loadable_signal::LoadableSignal,
};
