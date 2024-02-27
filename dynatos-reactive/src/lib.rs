//! Reactivity for `dynatos`

// Features
#![feature(unsize, coerce_unsized)]

// Modules
pub mod derived;
pub mod effect;
pub mod signal;
pub mod trigger;
pub mod with_default;

// Exports
pub use self::{
	derived::Derived,
	effect::{Effect, WeakEffect},
	signal::Signal,
	trigger::Trigger,
	with_default::{SignalWithDefault, WithDefault},
};

/// Signal get
#[extend::ext(name = SignalGet)]
pub impl<S> S
where
	S: SignalWith,
	S::Value: Copy,
{
	/// Gets the signal value, by copying it
	fn get(&self) -> S::Value {
		self.with(|value| *value)
	}
}

/// Signal cloned
#[extend::ext(name = SignalGetCloned)]
pub impl<S> S
where
	S: SignalWith,
	S::Value: Clone,
{
	/// Gets the signal value, by cloning it
	fn get_cloned(&self) -> S::Value {
		self.with(|value| value.clone())
	}
}

/// Signal with
pub trait SignalWith {
	/// Value type
	type Value: ?Sized;

	/// Uses the signal value
	fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Self::Value) -> O;
}

/// Signal set
pub trait SignalSet<Value> {
	/// Sets the signal value
	fn set(&self, new_value: Value);
}

/// Signal replace
pub trait SignalReplace<Value> {
	/// Replaces the signal value, returning the previous value
	fn replace(&self, new_value: Value) -> Value;
}

/// Signal update
pub trait SignalUpdate {
	/// Value type
	type Value: ?Sized;

	/// Updates the signal value
	fn update<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&mut Self::Value) -> O;
}
