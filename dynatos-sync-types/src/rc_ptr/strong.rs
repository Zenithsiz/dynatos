//! Strong pointer

// Imports
use {
	super::WeakRcPtr,
	core::{
		marker::Unsize,
		ops::{CoerceUnsized, Deref, DispatchFromDyn},
	},
};

type Inner<T> = cfg_select! {
	feature = "sync" => std::sync::Arc::<T>,
	_ => std::rc::Rc::<T>,
};


/// Reference-counted pointer
// Note: Named `RcPtr` to differentiate it
//       from the standard `Rc`.
#[derive(derive_more::Debug)]
#[debug("{_0:?}")]
pub struct RcPtr<T: ?Sized>(pub(crate) Inner<T>);

impl<T> RcPtr<T> {
	pub fn new(value: T) -> Self {
		Self(Inner::new(value))
	}
}

impl<T: ?Sized> RcPtr<T> {
	#[must_use]
	pub fn downgrade(this: &Self) -> WeakRcPtr<T> {
		WeakRcPtr(Inner::downgrade(&this.0))
	}

	#[must_use]
	pub fn strong_count(this: &Self) -> usize {
		Inner::strong_count(&this.0)
	}

	#[must_use]
	pub fn weak_count(this: &Self) -> usize {
		Inner::weak_count(&this.0)
	}

	#[must_use]
	pub fn as_ptr(this: &Self) -> *const T {
		Inner::as_ptr(&this.0)
	}
}

#[duplicate::duplicate_item(
	Generics FromTy RcTy;
	[T] [T] [T];
	[] [&'_ str] [str];
)]
impl<Generics> From<FromTy> for RcPtr<RcTy> {
	fn from(value: FromTy) -> Self {
		Self(Inner::from(value))
	}
}

impl<T: ?Sized> Clone for RcPtr<T> {
	fn clone(&self) -> Self {
		Self(Inner::clone(&self.0))
	}
}

impl<T: ?Sized> Deref for RcPtr<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<RcPtr<U>> for RcPtr<T> {}
impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<RcPtr<U>> for RcPtr<T> {}
