//! # Memo'd signals

// Imports
use {
	crate::{Effect, IMut, IMutExt, IMutRef, SignalBorrow, SignalWith, SyncBounds, Trigger},
	core::{
		fmt,
		marker::Unsize,
		ops::{CoerceUnsized, Deref},
	},
};

/// Memo signal.
///
/// See the module documentation for more information.
pub struct Memo<T, F: ?Sized> {
	/// Effect
	effect: Effect<EffectFn<T, F>>,
}

impl<T, F> Memo<T, F> {
	/// Creates a new memo'd signal
	#[track_caller]
	pub fn new(f: F) -> Self
	where
		T: PartialEq + 'static + SyncBounds,
		F: Fn() -> T + 'static + SyncBounds,
	{
		let value = IMut::new(None);
		let effect = Effect::new(EffectFn {
			trigger: Trigger::new(),
			value,
			f,
		});

		Self { effect }
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T>(IMutRef<'a, Option<T>>);

impl<T> Deref for BorrowRef<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Value wasn't initialized")
	}
}

impl<T: 'static, F: ?Sized> SignalBorrow for Memo<T, F> {
	type Ref<'a>
		= BorrowRef<'a, T>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		self.effect.inner_fn().trigger.gather_subscribers();

		let effect_fn = self.effect.inner_fn();
		let value = effect_fn.value.imut_read();
		BorrowRef(value)
	}
}

impl<T: 'static, F: ?Sized> SignalWith for Memo<T, F> {
	type Value<'a> = &'a T;

	#[track_caller]
	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(&value)
	}
}

impl<T, F: ?Sized> Clone for Memo<T, F> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
		}
	}
}

impl<T: fmt::Debug, F: ?Sized> fmt::Debug for Memo<T, F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let effect_fn = self.effect.inner_fn();
		f.debug_struct("Derived").field("value", &effect_fn.value).finish()
	}
}

impl<T, F1, F2> CoerceUnsized<Memo<T, F2>> for Memo<T, F1>
where
	F1: ?Sized + Unsize<F2>,
	F2: ?Sized,
{
}

/// Effect function
struct EffectFn<T, F: ?Sized> {
	/// Trigger
	trigger: Trigger,

	/// Value
	value: IMut<Option<T>>,

	/// Function
	f: F,
}

impl<T, F> FnOnce<()> for EffectFn<T, F>
where
	T: PartialEq + 'static,
	F: Fn() -> T,
{
	type Output = ();

	extern "rust-call" fn call_once(mut self, args: ()) -> Self::Output {
		self.call_mut(args);
	}
}
impl<T, F> FnMut<()> for EffectFn<T, F>
where
	T: PartialEq + 'static,
	F: Fn() -> T,
{
	extern "rust-call" fn call_mut(&mut self, args: ()) -> Self::Output {
		self.call(args);
	}
}
impl<T, F> Fn<()> for EffectFn<T, F>
where
	T: PartialEq + 'static,
	F: Fn() -> T,
{
	extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
		let new_value = (self.f)();
		let mut value = self.value.imut_write();

		// Write the new value, if it's different from the previous
		// Note: Since we're comparing against `Some(_)`, any `None` values
		//       will always be written to.
		let is_same = value.as_ref() == Some(&new_value);
		if !is_same {
			*value = Some(new_value);
			drop(value);
			self.trigger.trigger();
		}
	}
}
