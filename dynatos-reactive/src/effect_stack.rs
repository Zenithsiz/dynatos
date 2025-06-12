//! Effect stack

// Imports
use {
	crate::{Effect, EffectRun},
	core::{cell::RefCell, marker::Unsize},
};

/// Effect stack impl
type EffectStackImpl<F> = RefCell<Vec<Effect<F>>>;

/// Effect stack
#[thread_local]
static EFFECT_STACK: EffectStackImpl<dyn EffectRun> = EffectStackImpl::new(vec![]);

/// Pushes an effect to the stack.
pub fn push<F>(f: Effect<F>)
where
	F: ?Sized + Unsize<dyn EffectRun>,
{
	EFFECT_STACK.borrow_mut().push(f);
}

/// Pops an effect from the stack
pub fn pop() {
	EFFECT_STACK.borrow_mut().pop().expect("Missing added effect");
}

/// Returns the top effect of the stack
pub fn top() -> Option<Effect<dyn EffectRun>> {
	EFFECT_STACK.borrow().last().cloned()
}
