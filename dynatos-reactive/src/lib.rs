//! Reactivity for `dynatos`

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

// Modules
pub mod async_signal;
pub mod derived;
pub mod effect;
pub mod memo;
pub mod signal;
pub mod trigger;
pub mod with_default;

// Exports
pub use self::{
	async_signal::AsyncSignal,
	derived::Derived,
	effect::{Effect, WeakEffect},
	memo::Memo,
	signal::{
		Signal,
		SignalBorrow,
		SignalBorrowMut,
		SignalGet,
		SignalGetClone,
		SignalGetCloned,
		SignalGetCopy,
		SignalReplace,
		SignalSet,
		SignalSetWith,
		SignalUpdate,
		SignalWith,
	},
	trigger::{IntoSubscriber, Subscriber, Trigger, WeakTrigger},
	with_default::{SignalWithDefault, WithDefault},
};

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

		fn imut_try_read(&self) -> Option<IMutRef<'_, T>> {
			self.try_read()
		}

		fn imut_try_write(&self) -> Option<IMutRefMut<'_, T>> {
			self.try_write()
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

		fn imut_try_read(&self) -> Option<IMutRef<'_, T>> {
			self.try_borrow().ok()
		}

		fn imut_try_write(&self) -> Option<IMutRefMut<'_, T>> {
			self.try_borrow_mut().ok()
		}
	}
}

use private::*;

trait IMutExt<T: ?Sized> {
	fn imut_read(&self) -> IMutRef<'_, T>;
	fn imut_write(&self) -> IMutRefMut<'_, T>;

	fn imut_try_read(&self) -> Option<IMutRef<'_, T>>;
	fn imut_try_write(&self) -> Option<IMutRefMut<'_, T>>;
}
