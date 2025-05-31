//! Loadable values for `dynatos`

// Features
#![feature(
	try_trait_v2,
	never_type,
	extend_one,
	unboxed_closures,
	negative_impls,
	try_trait_v2_residual
)]

// Modules
pub mod loadable;
pub mod loadable_borrow;
pub mod loadable_signal;

// Exports
pub use self::{
	loadable::{IntoLoaded, IteratorLoadableExt, Loadable},
	loadable_borrow::{LoadableBorrow, LoadableBorrowMut, SignalBorrowLoadable, SignalBorrowMutLoadable},
	loadable_signal::LoadableSignal,
};
