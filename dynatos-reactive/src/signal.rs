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
	crate::{trigger::TriggerExec, ReactiveWorld, Trigger},
	core::{
		fmt,
		marker::Unsize,
		mem,
		ops::{CoerceUnsized, Deref, DerefMut},
	},
	dynatos_world::{IMut, IMutLike, IMutRef, IMutRefMut, Rc, RcLike, WorldDefault},
};

/// Inner
struct Inner<T: ?Sized, W: ReactiveWorld> {
	/// Trigger
	trigger: Trigger<W>,

	/// Value
	value: IMut<T, W>,
}

/// Signal
pub struct Signal<T: ?Sized, W: ReactiveWorld = WorldDefault> {
	/// Inner
	inner: Rc<Inner<T, W>, W>,
}

impl<T> Signal<T, WorldDefault> {
	/// Creates a new signal
	#[track_caller]
	pub fn new(value: T) -> Self
	where
		IMut<T, WorldDefault>: Sized,
	{
		Self::new_in(value, WorldDefault::default())
	}
}

impl<T, W: ReactiveWorld> Signal<T, W> {
	/// Creates a new signal in a world.
	#[track_caller]
	pub fn new_in(value: T, world: W) -> Self
	where
		IMut<T, W>: Sized,
	{
		let inner = Inner {
			value:   IMut::<_, W>::new(value),
			trigger: Trigger::new_in(world),
		};
		Self {
			inner: Rc::<_, W>::new(inner),
		}
	}
}

// TODO: Add `Signal::<dyn Any>::downcast` once we add `{T, U}: ?Sized` to the `CoerceUnsized` impl of `Inner`.
//       Use `Rc::downcast::<Inner<T>>(self.inner as Rc<dyn Any>)`

impl<T, U, W> CoerceUnsized<Signal<U, W>> for Signal<T, W>
where
	T: ?Sized + Unsize<U>,
	U: ?Sized,
	W: ReactiveWorld,
	Rc<Inner<T, W>, W>: CoerceUnsized<Rc<Inner<U, W>, W>>,
{
}

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, T: ?Sized + 'a, W: ReactiveWorld = WorldDefault>(IMutRef<'a, T, W>);

impl<T: ?Sized, W: ReactiveWorld> Deref for BorrowRef<'_, T, W> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: fmt::Debug, W: ReactiveWorld> fmt::Debug for BorrowRef<'_, T, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("BorrowRef").field(&*self.0).finish()
	}
}

impl<T: ?Sized + 'static, W: ReactiveWorld> SignalBorrow for Signal<T, W> {
	type Ref<'a>
		= BorrowRef<'a, T, W>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.inner.trigger.gather_subscribers();

		self.borrow_raw()
	}

	fn borrow_raw(&self) -> Self::Ref<'_> {
		let value = self.inner.value.read();
		BorrowRef(value)
	}
}

impl<T: 'static, W: ReactiveWorld> SignalReplace<T> for Signal<T, W> {
	type Value = T;

	fn replace(&self, new_value: T) -> Self::Value {
		mem::replace(&mut self.borrow_mut(), new_value)
	}

	fn replace_raw(&self, new_value: T) -> Self::Value {
		mem::replace(&mut self.borrow_mut_raw(), new_value)
	}
}

/// Reference type for [`SignalBorrowMut`] impl
pub struct BorrowRefMut<'a, T: ?Sized + 'a, W: ReactiveWorld = WorldDefault> {
	/// Value
	value: IMutRefMut<'a, T, W>,

	/// Trigger executor
	// Note: Must be dropped *after* `value`.
	_trigger_exec: Option<TriggerExec<W>>,
}

impl<T: ?Sized, W: ReactiveWorld> Deref for BorrowRefMut<'_, T, W> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<T: ?Sized, W: ReactiveWorld> DerefMut for BorrowRefMut<'_, T, W> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.value
	}
}

impl<T: fmt::Debug, W: ReactiveWorld> fmt::Debug for BorrowRefMut<'_, T, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("BorrowRefMut").field(&*self.value).finish()
	}
}

impl<T: ?Sized + 'static, W: ReactiveWorld> SignalBorrowMut for Signal<T, W> {
	type RefMut<'a>
		= BorrowRefMut<'a, T, W>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.inner.value.write();
		BorrowRefMut {
			value,
			_trigger_exec: Some(self.inner.trigger.exec()),
		}
	}

	fn borrow_mut_raw(&self) -> Self::RefMut<'_> {
		let value = self.inner.value.write();
		BorrowRefMut {
			value,
			_trigger_exec: None,
		}
	}
}


impl<T: ?Sized, W: ReactiveWorld> SignalSetDefaultImpl for Signal<T, W> {}
impl<T: ?Sized, W: ReactiveWorld> SignalGetDefaultImpl for Signal<T, W> {}
impl<T: ?Sized, W: ReactiveWorld> SignalGetClonedDefaultImpl for Signal<T, W> {}
impl<T: ?Sized, W: ReactiveWorld> SignalWithDefaultImpl for Signal<T, W> {}
impl<T: ?Sized, W: ReactiveWorld> SignalUpdateDefaultImpl for Signal<T, W> {}

impl<T: ?Sized, W: ReactiveWorld> Clone for Signal<T, W> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::<_, W>::clone(&self.inner),
		}
	}
}

impl<T: ?Sized + fmt::Debug, W: ReactiveWorld> fmt::Debug for Signal<T, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Signal")
			.field("value", &&*self.inner.value.read())
			.field("trigger", &self.inner.trigger)
			.finish()
	}
}

#[cfg(test)]
mod test {
	// Imports
	extern crate test;
	use {super::*, crate::Effect, core::array, test::Bencher, zutil_cloned::cloned};

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

	#[bench]
	fn clone_100(bencher: &mut Bencher) {
		let signals = array::from_fn::<_, 100, _>(|_| Signal::new(0_i32));
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
		let values = array::from_fn::<_, 100, _>(|_| 123_usize);
		bencher.iter(|| {
			for value in &values {
				test::black_box(*value);
			}
		});
	}

	#[bench]
	fn access_100(bencher: &mut Bencher) {
		let signals = array::from_fn::<_, 100, _>(|_| Signal::new(123_usize));
		bencher.iter(|| {
			for signal in &signals {
				test::black_box(signal.get());
			}
		});
	}

	/// Reference for `update_100_*`
	#[bench]
	fn update_100_value(bencher: &mut Bencher) {
		let mut values = array::from_fn::<_, 100, _>(|_| 123_usize);
		bencher.iter(|| {
			for value in &mut values {
				*value += 1;
				test::black_box(*value);
			}
		});
	}

	#[bench]
	fn update_100_empty(bencher: &mut Bencher) {
		let signals = array::from_fn::<_, 100, _>(|_| Signal::new(123_usize));
		bencher.iter(|| {
			for signal in &signals {
				signal.update(|value| *value += 1);
				test::black_box(signal);
			}
		});
	}
}
