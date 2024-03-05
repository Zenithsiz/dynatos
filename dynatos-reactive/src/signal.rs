//! Signal
//!
//! A read-write value that automatically updates
//! any subscribers when changed.

// Modules
pub mod ops;

// Exports
pub use ops::{
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
	crate::{effect, Trigger},
	std::{cell::RefCell, fmt, marker::Unsize, mem, ops::CoerceUnsized, rc::Rc},
};

/// Inner
struct Inner<T: ?Sized> {
	/// Trigger
	trigger: Trigger,

	/// Value
	value: RefCell<T>,
}

// TODO: Add `T: ?Sized, U: ?Sized` once `RefCell` supports it.
impl<T, U> CoerceUnsized<Inner<U>> for Inner<T> where T: CoerceUnsized<U> {}

/// Signal
pub struct Signal<T: ?Sized> {
	/// Inner
	inner: Rc<Inner<T>>,
}

impl<T> Signal<T> {
	/// Creates a new signal
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

impl<T: ?Sized, U: ?Sized> CoerceUnsized<Signal<U>> for Signal<T> where T: Unsize<U> {}

impl<T: ?Sized + 'static> SignalWith for Signal<T> {
	type Value<'a> = &'a T;

	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		if let Some(effect) = effect::running() {
			self.inner.trigger.add_subscriber(effect);
		}

		let value = self
			.inner
			.value
			.try_borrow()
			.expect("Cannot use signal value while updating");
		f(&value)
	}
}

impl<T: 'static> SignalReplace<T> for Signal<T> {
	fn replace(&self, new_value: T) -> T {
		self.update(|value| mem::replace(value, new_value))
	}
}

impl<T: ?Sized + 'static> SignalUpdate for Signal<T> {
	type Value<'a> = &'a mut T;

	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		// Update the value and get the output
		let output = {
			let mut value = self
				.inner
				.value
				.try_borrow_mut()
				.expect("Cannot update signal value while using it");
			f(&mut value)
		};

		// Then trigger our trigger
		self.inner.trigger.trigger();

		output
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
			.field("value", &*self.inner.value.borrow())
			.field("trigger", &self.inner.trigger)
			.finish()
	}
}

#[cfg(test)]
mod test {
	// Imports
	extern crate test;
	use {super::*, crate::SignalGet, std::mem, test::Bencher};

	#[bench]
	fn clone_100(bencher: &mut Bencher) {
		let signals = std::array::from_fn::<_, 100, _>(|_| Signal::new(0_i32));
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
		let values = std::array::from_fn::<_, 100, _>(|_| 123_usize);
		bencher.iter(|| {
			for value in &values {
				test::black_box(*value);
			}
		});
	}

	#[bench]
	fn access_100(bencher: &mut Bencher) {
		let signals = std::array::from_fn::<_, 100, _>(|_| Signal::new(123_usize));
		bencher.iter(|| {
			for signal in &signals {
				test::black_box(signal.get());
			}
		});
	}

	/// Reference for `update_100_*`
	#[bench]
	fn update_100_value(bencher: &mut Bencher) {
		let mut values = std::array::from_fn::<_, 100, _>(|_| 123_usize);
		bencher.iter(|| {
			for value in &mut values {
				*value += 1;
				test::black_box(*value);
			}
		});
	}

	#[bench]
	fn update_100_empty(bencher: &mut Bencher) {
		let signals = std::array::from_fn::<_, 100, _>(|_| Signal::new(123_usize));
		bencher.iter(|| {
			for signal in &signals {
				signal.update(|value| *value += 1);
				test::black_box(signal);
			}
		});
	}
}
