//! Loadable values for `dynatos`

#![feature(try_trait_v2, extend_one, unboxed_closures, negative_impls, try_trait_v2_residual)]

pub mod loadable;
pub mod loadable_borrow;
pub mod loadable_signal;

pub use self::{
	loadable::{IntoLoaded, IteratorLoadableExt, Loadable},
	loadable_borrow::{LoadableBorrow, LoadableBorrowMut, SignalBorrowLoadable, SignalBorrowMutLoadable},
	loadable_signal::LoadableSignal,
};
