//! Storage

// Imports
use crate::{Effect, EffectRun, Signal, SignalSet};

/// Signal storage.
#[derive(Debug)]
pub struct SignalStorage<T> {
	/// Signal
	// TODO: Allow the user to specify another type of signal?
	signal: Signal<T>,

	/// Write-back effect
	// TODO: Not use dynamic dispatch here
	write_back_effect: Effect<dyn EffectRun>,
}

impl<T> SignalStorage<T> {
	/// Creates a new signal storage
	pub(crate) fn new(signal: Signal<T>, write_back_effect: Effect<dyn EffectRun>) -> Self {
		Self {
			signal,
			write_back_effect,
		}
	}

	/// Clones the signal in storage
	#[must_use]
	pub fn signal(&self) -> Signal<T> {
		self.signal.clone()
	}

	/// Sets the value of the signal in storage.
	///
	/// Suppresses the write-back effect during.
	pub fn set(&self, new_value: T)
	where
		T: 'static,
	{
		self.write_back_effect.suppressed(|| self.signal.set(new_value));
	}
}
