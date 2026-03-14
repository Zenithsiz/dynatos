//! [`SignalReplace`]

// Imports
use crate::effect;

/// Signal replace
pub trait SignalReplace<T> {
	type Value;

	/// Replaces the signal value, returning the previous value
	#[track_caller]
	fn replace(&self, new_value: T) -> Self::Value;

	/// Replaces the signal value, returning the previous value without updating dependencies.
	#[track_caller]
	fn replace_no_run(&self, new_value: T) -> Self::Value {
		effect::with_no_run(|| self.replace(new_value))
	}
}
