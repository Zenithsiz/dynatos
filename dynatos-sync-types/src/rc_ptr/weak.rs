//! Weak pointer

// Imports
use {
	super::RcPtr,
	core::{
		marker::Unsize,
		ops::{CoerceUnsized, DispatchFromDyn},
	},
};

pub type Inner<T> = cfg_select! {
	feature = "sync" => std::sync::Weak::<T>,
	_ => std::rc::Weak::<T>,
};

/// Weak reference-counted pointer
// Note: Named `WeakRcPtr` to differentiate it
//       from the standard `Weak`.
#[derive(derive_more::Debug)]
#[debug("{_0:?}")]
pub struct WeakRcPtr<T: ?Sized>(pub(crate) Inner<T>);

impl<T> WeakRcPtr<T> {
	#[must_use]
	pub const fn new() -> Self {
		Self(Inner::new())
	}
}

impl<T: ?Sized> WeakRcPtr<T> {
	#[must_use]
	pub fn upgrade(&self) -> Option<RcPtr<T>> {
		self.0.upgrade().map(RcPtr)
	}

	#[must_use]
	pub fn as_ptr(this: &Self) -> *const T {
		Inner::as_ptr(&this.0)
	}
}

impl<T: ?Sized> Clone for WeakRcPtr<T> {
	fn clone(&self) -> Self {
		Self(Inner::clone(&self.0))
	}
}

impl<T> Default for WeakRcPtr<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<WeakRcPtr<U>> for WeakRcPtr<T> {}
impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<WeakRcPtr<U>> for WeakRcPtr<T> {}
