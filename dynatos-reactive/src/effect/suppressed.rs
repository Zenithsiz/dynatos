//! Suppressed

// Imports
use super::Effect;

/// Effect suppression.
///
/// While this type is alive, the corresponding
/// effect will be suppressed and not be added
/// to the run queue.
#[must_use = "The effect is only suppressed while this value is value"]
pub struct EffectSuppressed<'a, F: ?Sized> {
	/// Effect
	effect: &'a Effect<F>,

	/// Previous suppression value
	prev_suppressed: bool,
}

impl<'a, F: ?Sized> EffectSuppressed<'a, F> {
	/// Creates a new dependency gatherer
	pub fn new(effect: &'a Effect<F>) -> Self {
		// Set the effect as suppressed
		let prev_suppressed = effect.inner.suppressed.get();
		effect.inner.suppressed.set(true);

		Self {
			effect,
			prev_suppressed,
		}
	}
}

impl<F: ?Sized> Drop for EffectSuppressed<'_, F> {
	fn drop(&mut self) {
		// Set it back as it was previously
		self.effect.inner.suppressed.set(self.prev_suppressed);
	}
}
