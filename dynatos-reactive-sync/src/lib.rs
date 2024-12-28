//! Helper crate for `dynatos` reactivity.
//!
//! Helps select single-threaded vs multi-threaded primitives.

// TODO: Also include `Cell<T>` vs `Atomic<T>`.

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
	pub type IMutRefMapped<'a, T> = parking_lot::MappedRwLockReadGuard<'a, T>;
	pub type IMutRefMut<'a, T> = parking_lot::RwLockWriteGuard<'a, T>;

	impl<T: ?Sized> crate::IMutExt<T> for IMut<T> {
		fn imut_read(&self) -> IMutRef<'_, T> {
			self.read()
		}

		fn imut_write(&self) -> IMutRefMut<'_, T> {
			self.write()
		}
	}

	impl<'a, T: ?Sized> crate::IMutRefMutExt<'a, T> for IMutRefMut<'a, T> {
		fn imut_downgrade(this: Self) -> IMutRef<'a, T> {
			IMutRefMut::downgrade(this)
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
	pub type IMutRefMapped<'a, T> = core::cell::Ref<'a, T>;

	#[derive(derive_more::Deref, derive_more::DerefMut, derive_more::Debug)]
	#[debug("{borrow:?}")]
	pub struct IMutRefMut<'a, T: ?Sized> {
		/// Borrow
		#[deref(forward)]
		#[deref_mut]
		borrow: core::cell::RefMut<'a, T>,

		/// Original refcell
		// Note: TheThis field is necessary for downgrading.
		refcell: &'a IMut<T>,
	}

	impl<T: ?Sized> crate::IMutExt<T> for IMut<T> {
		fn imut_read(&self) -> IMutRef<'_, T> {
			self.borrow()
		}

		fn imut_write(&self) -> IMutRefMut<'_, T> {
			IMutRefMut {
				borrow:  self.borrow_mut(),
				refcell: self,
			}
		}
	}

	impl<'a, T: ?Sized> crate::IMutRefMutExt<'a, T> for IMutRefMut<'a, T> {
		fn imut_downgrade(this: Self) -> IMutRef<'a, T> {
			// Note: RefCell is single threaded, so there are no races here
			drop(this.borrow);
			this.refcell.borrow()
		}
	}
}

pub use private::*;

/// Extension methods for the inner-mutability types.
pub trait IMutExt<T: ?Sized> {
	fn imut_read(&self) -> IMutRef<'_, T>;
	fn imut_write(&self) -> IMutRefMut<'_, T>;
}

/// Extension methods for the inner-mutability mutable references
pub trait IMutRefMutExt<'a, T: ?Sized> {
	fn imut_downgrade(this: Self) -> IMutRef<'a, T>;
}
