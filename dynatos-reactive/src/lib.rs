//! Reactivity for [`dynatos`]

// Modules
pub mod effect;
pub mod signal;
pub mod with_default;

// Exports
pub use self::{
	effect::{Effect, WeakEffect},
	signal::Signal,
	with_default::{SignalWithDefault, WithDefault},
};

/// Signal get
pub trait SignalGet {
	/// Value type
	type Value;

	/// Gets the signal value, by copying it
	fn get(&self) -> Self::Value;
}

/// Signal with
pub trait SignalWith {
	/// Value type
	type Value;

	/// Uses the signal value
	fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Self::Value) -> O;
}

/// Signal set
pub trait SignalSet {
	/// Value type
	type Value;

	/// Sets the signal value
	fn set(&self, new_value: Self::Value);
}

/// Signal replace
pub trait SignalReplace {
	/// Value type
	type Value;

	/// Replaces the signal value, returning the previous value
	fn replace(&self, new_value: Self::Value) -> Self::Value;
}

/// Signal update
pub trait SignalUpdate {
	/// Value type
	type Value;

	/// Updates the signal value
	fn update<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&mut Self::Value) -> O;
}
