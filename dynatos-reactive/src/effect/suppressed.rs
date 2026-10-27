//! Suppressed

use {super::Effect, core::sync::atomic};

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
		let prev_suppressed = effect.inner.suppressed.swap(true, atomic::Ordering::AcqRel);

		Self {
			effect,
			prev_suppressed,
		}
	}
}

impl<F: ?Sized> Drop for EffectSuppressed<'_, F> {
	fn drop(&mut self) {
		// Set it back as it was previously
		self.effect
			.inner
			.suppressed
			.store(self.prev_suppressed, atomic::Ordering::Release);
	}
}

#[cfg(test)]
mod tests {
	use {super::*, crate::Trigger, core::cell::Cell, std::rc::Rc, zutil_cloned::cloned};

	#[test]
	fn suppressed() {
		let trigger = Trigger::new();
		let times_ran = Rc::new(Cell::new(0));

		#[cloned(trigger, times_ran)]
		let effect = Effect::new(move || {
			trigger.gather_subs();
			times_ran.update(|ran| ran + 1);
		});
		assert_eq!(times_ran.get(), 1);

		// Executing the trigger should update the effect
		trigger.exec();
		assert_eq!(times_ran.get(), 2);

		// Running when suppressed shouldn't execute it.
		{
			let _suppressed = effect.suppress();
			trigger.exec();
		}
		assert_eq!(times_ran.get(), 2);

		// Ensure that the effect doesn't run if there's an execution on the stack
		{
			let dummy_trigger = Trigger::new();
			let exec = dummy_trigger.exec();
			trigger.exec();
			assert_eq!(times_ran.get(), 2);
			drop(exec);
			assert_eq!(times_ran.get(), 3);
		}

		// Suppressing should work when execution is delayed
		{
			let dummy_trigger = Trigger::new();
			let _exec = dummy_trigger.exec();
			{
				let _suppressed = effect.suppress();
				trigger.exec();
			}
		}
		assert_eq!(times_ran.get(), 3);

		// Ensure that the trigger didn't lose the effect as a subscriber
		trigger.exec();
		assert_eq!(times_ran.get(), 4);
	}
}
