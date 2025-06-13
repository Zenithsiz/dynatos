//! Dependency gatherer

use {
	super::{Effect, EffectRun},
	crate::effect_stack,
	core::marker::PhantomData,
};

/// Effect dependency gatherer.
///
/// While this type is alive, any signals used will
/// be added as a dependency.
pub struct EffectDepsGatherer<'a>(PhantomData<&'a ()>);

impl<'a> EffectDepsGatherer<'a> {
	/// Creates a new dependency gatherer
	#[must_use]
	pub fn new<F>(effect: &'a Effect<F>) -> Self
	where
		F: ?Sized + EffectRun,
	{
		// Push the effect onto the stack
		effect_stack::push(effect.clone().unsize());

		Self(PhantomData)
	}
}

impl Drop for EffectDepsGatherer<'_> {
	fn drop(&mut self) {
		// Pop our effect from the stack
		effect_stack::pop();
	}
}
