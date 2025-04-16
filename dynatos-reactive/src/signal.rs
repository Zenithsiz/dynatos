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
	crate::{
		trigger::TriggerWorld,
		world::{IMut, IMutLike, IMutRef, IMutRefMut, Rc, RcLike},
		Trigger,
		World,
		WorldDefault,
	},
	core::{
		fmt,
		marker::Unsize,
		mem,
		ops::{CoerceUnsized, Deref, DerefMut},
	},
};

/// World for [`Signal`]
pub trait SignalWorld = World + TriggerWorld;

/// Inner
struct Inner<T: ?Sized, W: SignalWorld> {
	/// Trigger
	trigger: Trigger<W>,

	/// Value
	value: IMut<T, W>,
}

/// Signal
pub struct Signal<T: ?Sized, W: SignalWorld = WorldDefault> {
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

impl<T, W: SignalWorld> Signal<T, W> {
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
	W: SignalWorld,
	Rc<Inner<T, W>, W>: CoerceUnsized<Rc<Inner<U, W>, W>>,
{
}

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, T: ?Sized + 'a, W: SignalWorld = WorldDefault>(IMutRef<'a, T, W>);

impl<T: ?Sized, W: SignalWorld> Deref for BorrowRef<'_, T, W> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'a, T: fmt::Debug, W: SignalWorld> fmt::Debug for BorrowRef<'a, T, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("BorrowRef").field(&*self.0).finish()
	}
}

impl<T: ?Sized + 'static, W: SignalWorld> SignalBorrow for Signal<T, W> {
	type Ref<'a>
		= BorrowRef<'a, T, W>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		self.inner.trigger.gather_subscribers();

		let borrow = self.inner.value.read();
		BorrowRef(borrow)
	}
}

impl<T: ?Sized + 'static, W: SignalWorld> SignalWith for Signal<T, W> {
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

impl<T: 'static, W: SignalWorld> SignalReplace<T> for Signal<T, W> {
	fn replace(&self, new_value: T) -> T {
		mem::replace(&mut self.borrow_mut(), new_value)
	}
}

/// Triggers on `Drop`
// Note: We need this wrapper because `BorrowRefMut::value` must
//       already be dropped when we run the trigger, which we
//       can't do if we implement `Drop` on `BorrowRefMut`.
#[derive(Debug)]
struct TriggerOnDrop<'a, W: SignalWorld>(pub &'a Trigger<W>);

impl<W: SignalWorld> Drop for TriggerOnDrop<'_, W> {
	fn drop(&mut self) {
		self.0.trigger();
	}
}

/// Reference type for [`SignalBorrowMut`] impl
pub struct BorrowRefMut<'a, T: ?Sized + 'a, W: SignalWorld = WorldDefault> {
	/// Value
	value: IMutRefMut<'a, T, W>,

	/// Trigger on drop
	// Note: Must be dropped *after* `value`.
	_trigger_on_drop: TriggerOnDrop<'a, W>,
}

impl<T: ?Sized, W: SignalWorld> Deref for BorrowRefMut<'_, T, W> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<T: ?Sized, W: SignalWorld> DerefMut for BorrowRefMut<'_, T, W> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.value
	}
}

impl<'a, T: fmt::Debug, W: SignalWorld> fmt::Debug for BorrowRefMut<'a, T, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("BorrowRef").field(&*self.value).finish()
	}
}

impl<T: ?Sized + 'static, W: SignalWorld> SignalBorrowMut for Signal<T, W> {
	type RefMut<'a>
		= BorrowRefMut<'a, T, W>
	where
		Self: 'a;

	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.inner.value.write();
		BorrowRefMut {
			value,
			_trigger_on_drop: TriggerOnDrop(&self.inner.trigger),
		}
	}
}


impl<T: ?Sized + 'static, W: SignalWorld> SignalUpdate for Signal<T, W> {
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

impl<T, W: SignalWorld> Clone for Signal<T, W> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::<_, W>::clone(&self.inner),
		}
	}
}

impl<T: fmt::Debug, W: SignalWorld> fmt::Debug for Signal<T, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Signal")
			.field("value", &*self.inner.value.read())
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
