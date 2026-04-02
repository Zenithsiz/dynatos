//! Types for synchronization

// Features
#![feature(
	cfg_select,
	nonpoison_rwlock,
	nonpoison_mutex,
	sync_nonpoison,
	trait_alias,
	decl_macro,
	macro_attr,
	unsize,
	coerce_unsized,
	dispatch_from_dyn
)]
// Lints
#![expect(clippy::absolute_paths, reason = "It's easier when working with features")]

// Modules
mod bounds;
mod cell;
mod imut;
mod imut_rw;
mod lazy;
mod once;
mod rc_ptr;
mod thread_local;

// Exports
pub use self::{
	bounds::SyncBounds,
	cell::*,
	imut::{IMut, IMutRef},
	imut_rw::{IMutRw, IMutRwRef, IMutRwRefMut},
	lazy::LazyCell,
	once::OnceCell,
	rc_ptr::{RcPtr, WeakRcPtr},
	thread_local::thread_local_or_global,
};
