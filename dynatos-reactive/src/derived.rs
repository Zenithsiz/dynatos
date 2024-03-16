//! # Derived signals
//!
//! A derived signal, [`Derived`], is a signal that caches a reactive function's result, that is,
//! a function that depends on other signals.
//!
//! This is useful for splitting up an effect that requires computing multiple expensive operations,
//! to avoid needless re-computing certain values when others change.
//!
//! ## Examples
//! Without using a derived, whenever any dependent signals of `expensive_operation1` or
//! `expensive_operation2` are updated, then they will both be re-run due to `my_value`
//! requiring an update.
//! ```rust,no_run
//! // Pretend these are expensive operations
//! let expensive_operation1 = move || 1;
//! let expensive_operation2 = move || 2;
//! let my_value = move || expensive_operation1() + expensive_operation2();
//! ```
//!
//! Meanwhile, when using [`Derived`], you can cache each value, so that any updates
//! to one of the signals doesn't re-compute the other.
//! ```rust,no_run
//! use dynatos_reactive::{Derived, SignalGet};
//! let expensive_operation1 = Derived::new(move || 1);
//! let expensive_operation2 = Derived::new(move || 2);
//! let my_value = move || expensive_operation1.get() + expensive_operation2.get();
//! ```
//!
//! It's important to note that this isn't free however, as [`Derived`] needs to
//! not only store the latest value, it also needs to create an effect that is re-run
//! each time any dependencies are updated.

// Imports
use {
	crate::{effect, Effect, SignalBorrow, SignalWith, Trigger},
	core::{
		cell::{self, RefCell},
		fmt,
		marker::Unsize,
		ops::{CoerceUnsized, Deref},
	},
};

/// Derived signal.
///
/// See the module documentation for more information.
pub struct Derived<T, F: ?Sized> {
	/// Effect
	effect: Effect<EffectFn<T, F>>,
}

impl<T, F> Derived<T, F> {
	/// Creates a new derived signal
	pub fn new(f: F) -> Self
	where
		T: 'static,
		F: Fn() -> T + 'static,
	{
		let value = RefCell::new(None);
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
pub struct BorrowRef<'a, T>(cell::Ref<'a, Option<T>>);

impl<'a, T> Deref for BorrowRef<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Value wasn't initialized")
	}
}

impl<T: 'static, F: ?Sized> SignalBorrow for Derived<T, F> {
	type Ref<'a> = BorrowRef<'a, T>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		if let Some(effect) = effect::running() {
			self.effect.inner_fn().trigger.add_subscriber(effect);
		}

		let effect_fn = self.effect.inner_fn();
		let value = effect_fn.value.borrow();
		BorrowRef(value)
	}
}

impl<T: 'static, F: ?Sized> SignalWith for Derived<T, F> {
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

impl<T, F: ?Sized> Clone for Derived<T, F> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
		}
	}
}

impl<T: fmt::Debug, F: ?Sized> fmt::Debug for Derived<T, F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let effect_fn = self.effect.inner_fn();
		f.debug_struct("Derived").field("value", &effect_fn.value).finish()
	}
}

impl<T, F1, F2> CoerceUnsized<Derived<T, F2>> for Derived<T, F1>
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
	value: RefCell<Option<T>>,

	/// Function
	f: F,
}

impl<T, F> FnOnce<()> for EffectFn<T, F>
where
	T: 'static,
	F: Fn() -> T,
{
	type Output = ();

	extern "rust-call" fn call_once(mut self, args: ()) -> Self::Output {
		self.call_mut(args);
	}
}
impl<T, F> FnMut<()> for EffectFn<T, F>
where
	T: 'static,
	F: Fn() -> T,
{
	extern "rust-call" fn call_mut(&mut self, args: ()) -> Self::Output {
		self.call(args);
	}
}
impl<T, F> Fn<()> for EffectFn<T, F>
where
	T: 'static,
	F: Fn() -> T,
{
	extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
		*self.value.borrow_mut() = Some((self.f)());
		self.trigger.trigger();
	}
}
