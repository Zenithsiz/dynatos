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

// TODO: Make `Derived` always `usize`-sized
//       by merging the `effect::Inner` and `signal::Inner` somehow?

// Imports
use {
	crate::{Effect, Signal, SignalSet, SignalWith},
	std::fmt,
};

/// Derived signal.
///
/// See the module documentation for more information.
pub struct Derived<T> {
	/// Effect
	effect: Effect<dyn Fn()>,

	/// Value
	value: Signal<Option<T>>,
}

impl<T> Derived<T> {
	/// Creates a new derived signal
	pub fn new<F>(f: F) -> Self
	where
		T: 'static,
		F: Fn() -> T + 'static,
	{
		let value = Signal::new(None);
		let effect = Effect::new({
			let value = value.clone();
			move || value.set(Some(f()))
		});

		Self { effect, value }
	}
}

impl<T> SignalWith for Derived<T> {
	type Value = T;

	fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Self::Value) -> O,
	{
		self.value.with(|value| {
			let value = value.as_ref().expect("Value wasn't initialized");
			f(value)
		})
	}
}

impl<T> Clone for Derived<T> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
			value:  self.value.clone(),
		}
	}
}

impl<T: fmt::Debug> fmt::Debug for Derived<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Derived")
			.field("effect", &self.effect)
			.field("value", &self.value)
			.finish()
	}
}
