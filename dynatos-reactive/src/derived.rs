//! Derived signal

// TODO: Make `Derived` always `usize`-sized
//       by merging the `effect::Inner` and `signal::Inner` somehow?

// Imports
use {
	crate::{Effect, Signal, SignalGet, SignalSet, SignalWith},
	std::fmt,
};

/// Derived signal
pub struct Derived<T> {
	/// Effect
	effect: Effect,

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

impl<T> SignalGet for Derived<T>
where
	T: Copy,
{
	type Value = T;

	fn get(&self) -> Self::Value {
		self.with(|value| *value)
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
