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
	SignalGetCopy,
	SignalReplace,
	SignalSet,
	SignalSetWith,
	SignalUpdate,
	SignalWith,
};

// Imports
use {
	crate::{IMut, IMutExt, IMutRef, IMutRefMut, Rc, Trigger},
	core::{
		fmt,
		marker::Unsize,
		mem,
		ops::{CoerceUnsized, Deref, DerefMut},
	},
};

/// Inner
struct Inner<T: ?Sized> {
	/// Trigger
	trigger: Trigger,

	/// Value
	value: IMut<T>,
}

// TODO: Add `T: ?Sized, U: ?Sized` once `RwLock` supports it.
impl<T, U> CoerceUnsized<Inner<U>> for Inner<T>
where
	T: CoerceUnsized<U>,
	IMut<T>: CoerceUnsized<IMut<U>>,
{
}

/// Signal
pub struct Signal<T: ?Sized> {
	/// Inner
	inner: Rc<Inner<T>>,
}

impl<T> Signal<T> {
	/// Creates a new signal
	#[track_caller]
	pub fn new(value: T) -> Self {
		let inner = Inner {
			value:   IMut::new(value),
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
#[derive(Debug)]
pub struct BorrowRef<'a, T: ?Sized>(IMutRef<'a, T>);

impl<T: ?Sized> Deref for BorrowRef<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: ?Sized + 'static> SignalBorrow for Signal<T> {
	type Ref<'a>
		= BorrowRef<'a, T>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		self.inner.trigger.gather_subscribers();

		let borrow = self
			.inner
			.value
			.imut_try_read()
			.expect("Cannot use signal value while updating");
		BorrowRef(borrow)
	}
}

impl<T: ?Sized + 'static> SignalWith for Signal<T> {
	type Value<'a> = &'a T;

	#[track_caller]
	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(&*value)
	}
}

impl<T: 'static> SignalReplace<T> for Signal<T> {
	fn replace(&self, new_value: T) -> T {
		mem::replace(&mut self.borrow_mut(), new_value)
	}
}

/// Triggers on `Drop`
// Note: We need this wrapper because `BorrowRefMut::value` must
//       already be dropped when we run the trigger, which we
//       can't do if we implement `Drop` on `BorrowRefMut`.
#[derive(Debug)]
pub(crate) struct TriggerOnDrop<'a>(pub &'a Trigger);

impl Drop for TriggerOnDrop<'_> {
	fn drop(&mut self) {
		self.0.trigger();
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a, T: ?Sized> {
	/// Value
	value: IMutRefMut<'a, T>,

	/// Trigger on drop
	// Note: Must be dropped *after* `value`.
	_trigger_on_drop: TriggerOnDrop<'a>,
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

impl<T: ?Sized + 'static> SignalBorrowMut for Signal<T> {
	type RefMut<'a>
		= BorrowRefMut<'a, T>
	where
		Self: 'a;

	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self
			.inner
			.value
			.imut_try_write()
			.expect("Cannot update signal value while using it");
		BorrowRefMut {
			value,
			_trigger_on_drop: TriggerOnDrop(&self.inner.trigger),
		}
	}
}


impl<T: ?Sized + 'static> SignalUpdate for Signal<T> {
	type Value<'a> = &'a mut T;

	#[track_caller]
	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut value = self.borrow_mut();
		f(&mut *value)
	}
}

impl<T> Clone for Signal<T> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

impl<T: fmt::Debug> fmt::Debug for Signal<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Signal")
			.field("value", &*self.inner.value.imut_read())
			.field("trigger", &self.inner.trigger)
			.finish()
	}
}

#[cfg(test)]
mod test {
	// Imports
	extern crate test;
	use {super::*, test::Bencher};

	#[bench]
	fn clone_100(bencher: &mut Bencher) {
		let signals = core::array::from_fn::<_, 100, _>(|_| Signal::new(0_i32));
		bencher.iter(|| {
			for signal in &signals {
				let signal = test::black_box(signal.clone());
				mem::forget(signal);
			}
		});
	}

	/// Reference for [`access_100`]
	#[bench]
	fn access_100_value(bencher: &mut Bencher) {
		let values = core::array::from_fn::<_, 100, _>(|_| 123_usize);
		bencher.iter(|| {
			for value in &values {
				test::black_box(*value);
			}
		});
	}

	#[bench]
	fn access_100(bencher: &mut Bencher) {
		let signals = core::array::from_fn::<_, 100, _>(|_| Signal::new(123_usize));
		bencher.iter(|| {
			for signal in &signals {
				test::black_box(signal.get());
			}
		});
	}

	/// Reference for `update_100_*`
	#[bench]
	fn update_100_value(bencher: &mut Bencher) {
		let mut values = core::array::from_fn::<_, 100, _>(|_| 123_usize);
		bencher.iter(|| {
			for value in &mut values {
				*value += 1;
				test::black_box(*value);
			}
		});
	}

	#[bench]
	fn update_100_empty(bencher: &mut Bencher) {
		let signals = core::array::from_fn::<_, 100, _>(|_| Signal::new(123_usize));
		bencher.iter(|| {
			for signal in &signals {
				signal.update(|value| *value += 1);
				test::black_box(signal);
			}
		});
	}
}
