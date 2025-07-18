//! Signal
//!
//! A read-write value that automatically updates
//! any subscribers when changed.

// Modules
pub mod ops;

// Exports
pub use ops::{
	SignalBorrow,
	SignalBorrowMut,
	SignalGet,
	SignalGetClone,
	SignalGetCloned,
	SignalGetClonedDefaultImpl,
	SignalGetCopy,
	SignalGetDefaultImpl,
	SignalReplace,
	SignalSet,
	SignalSetDefaultImpl,
	SignalSetWith,
	SignalUpdate,
	SignalUpdateDefaultImpl,
	SignalWith,
	SignalWithDefaultImpl,
};

// Imports
use {
	crate::{trigger::TriggerExec, Trigger},
	core::{
		cell::{self, RefCell},
		fmt,
		marker::Unsize,
		mem,
		ops::{CoerceUnsized, Deref, DerefMut},
	},
	std::rc::Rc,
};

/// Inner
struct Inner<T: ?Sized> {
	/// Trigger
	trigger: Trigger,

	/// Value
	value: RefCell<T>,
}

/// Signal
pub struct Signal<T: ?Sized> {
	/// Inner
	inner: Rc<Inner<T>>,
}

impl<T> Signal<T> {
	/// Creates a new signal.
	#[track_caller]
	pub fn new(value: T) -> Self {
		let inner = Inner {
			value:   RefCell::new(value),
			trigger: Trigger::new(),
		};
		Self { inner: Rc::new(inner) }
	}
}

// TODO: Add `Signal::<dyn Any>::downcast` once we add `{T, U}: ?Sized` to the `CoerceUnsized` impl of `Inner`.
//       Use `Rc::downcast::<Inner<T>>(self.inner as Rc<dyn Any>)`

impl<T, U> CoerceUnsized<Signal<U>> for Signal<T>
where
	T: ?Sized + Unsize<U>,
	U: ?Sized,
{
}

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, T: ?Sized + 'a>(cell::Ref<'a, T>);

impl<T: ?Sized> Deref for BorrowRef<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[coverage(off)]
impl<T: fmt::Debug> fmt::Debug for BorrowRef<'_, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("BorrowRef").field(&*self.0).finish()
	}
}

impl<T: ?Sized + 'static> SignalBorrow for Signal<T> {
	type Ref<'a>
		= BorrowRef<'a, T>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.inner.trigger.gather_subs();

		let value = self.inner.value.borrow();
		BorrowRef(value)
	}
}

impl<T: 'static> SignalReplace<T> for Signal<T> {
	type Value = T;

	fn replace(&self, new_value: T) -> Self::Value {
		mem::replace(&mut self.borrow_mut(), new_value)
	}
}

/// Reference type for [`SignalBorrowMut`] impl
pub struct BorrowRefMut<'a, T: ?Sized + 'a> {
	/// Value
	value: cell::RefMut<'a, T>,

	/// Trigger executor
	// Note: Must be dropped *after* `value`.
	_trigger_exec: Option<TriggerExec>,
}

impl<T: ?Sized> Deref for BorrowRefMut<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<T: ?Sized> DerefMut for BorrowRefMut<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.value
	}
}

#[coverage(off)]
impl<T: fmt::Debug> fmt::Debug for BorrowRefMut<'_, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("BorrowRefMut").field(&*self.value).finish()
	}
}

impl<T: ?Sized + 'static> SignalBorrowMut for Signal<T> {
	type RefMut<'a>
		= BorrowRefMut<'a, T>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.inner.value.borrow_mut();
		BorrowRefMut {
			value,
			_trigger_exec: self.inner.trigger.exec(),
		}
	}
}


impl<T: ?Sized> SignalSetDefaultImpl for Signal<T> {}
impl<T: ?Sized> SignalGetDefaultImpl for Signal<T> {}
impl<T: ?Sized> SignalGetClonedDefaultImpl for Signal<T> {}
impl<T: ?Sized> SignalWithDefaultImpl for Signal<T> {}
impl<T: ?Sized> SignalUpdateDefaultImpl for Signal<T> {}

impl<T: ?Sized> Clone for Signal<T> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

#[coverage(off)]
impl<T: ?Sized + fmt::Debug> fmt::Debug for Signal<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Signal")
			.field("value", &&*self.inner.value.borrow())
			.field("trigger", &self.inner.trigger)
			.finish()
	}
}

#[cfg(test)]
mod tests {
	// Imports
	use {super::*, crate::Effect, zutil_cloned::cloned};

	#[test]
	fn multiple_mut() {
		let a = Signal::new(1_i32);
		let b = Signal::new(2_i32);

		#[cloned(a, b)]
		let _effect = Effect::new(move || {
			a.get();
			b.get();
		});

		let _a = a.borrow_mut();
		let _b = b.borrow_mut();
	}
}
