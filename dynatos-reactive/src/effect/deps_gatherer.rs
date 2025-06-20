//! Dependency gatherer

use {
	super::{Effect, EffectRun},
	crate::effect_stack,
	core::marker::PhantomData,
};

// TODO: Add type parameter `F`?

/// Effect dependency gatherer.
///
/// While this type is alive, any signals used will
/// be added as a dependency.
pub struct EffectDepsGatherer<'a, F: ?Sized>(PhantomData<&'a Effect<F>>);

impl<'a, F: ?Sized> EffectDepsGatherer<'a, F> {
	/// Creates a new dependency gatherer
	#[must_use]
	pub fn new(effect: &'a Effect<F>) -> Self
	where
		F: EffectRun,
	{
		// Push the effect onto the stack
		effect_stack::push(effect.clone().unsize());

		Self(PhantomData)
	}
}

impl<F: ?Sized> Drop for EffectDepsGatherer<'_, F> {
	fn drop(&mut self) {
		// Pop our effect from the stack
		effect_stack::pop();
	}
}
