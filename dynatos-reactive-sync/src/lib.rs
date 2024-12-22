//! Helper crate for `dynatos` reactivity.
//!
//! Helps select single-threaded vs multi-threaded primitives.

// Features
#![feature(
	unsize,
	coerce_unsized,
	unboxed_closures,
	fn_traits,
	test,
	thread_local,
	cfg_match,
	trait_alias
)]

#[cfg(feature = "sync")]
mod private {
	pub trait SyncBounds = Send + Sync;
	pub type Rc<T> = std::sync::Arc<T>;
	pub type Weak<T> = std::sync::Weak<T>;
	pub type IMut<T> = parking_lot::RwLock<T>;
	pub type IMutRef<'a, T> = parking_lot::RwLockReadGuard<'a, T>;
	pub type IMutRefMut<'a, T> = parking_lot::RwLockWriteGuard<'a, T>;

	impl<T: ?Sized> crate::IMutExt<T> for IMut<T> {
		fn imut_read(&self) -> IMutRef<'_, T> {
			self.read()
		}

		fn imut_write(&self) -> IMutRefMut<'_, T> {
			self.write()
		}
	}
}

#[cfg(not(feature = "sync"))]
mod private {
	pub trait SyncBounds =;
	pub type Rc<T> = std::rc::Rc<T>;
	pub type Weak<T> = std::rc::Weak<T>;
	pub type IMut<T> = core::cell::RefCell<T>;
	pub type IMutRef<'a, T> = core::cell::Ref<'a, T>;
	pub type IMutRefMut<'a, T> = core::cell::RefMut<'a, T>;

	impl<T: ?Sized> crate::IMutExt<T> for IMut<T> {
		fn imut_read(&self) -> IMutRef<'_, T> {
			self.borrow()
		}

		fn imut_write(&self) -> IMutRefMut<'_, T> {
			self.borrow_mut()
		}
	}
}

pub use private::*;

/// Extension methods for the inner-mutability types.
pub trait IMutExt<T: ?Sized> {
	fn imut_read(&self) -> IMutRef<'_, T>;
	fn imut_write(&self) -> IMutRefMut<'_, T>;
}
