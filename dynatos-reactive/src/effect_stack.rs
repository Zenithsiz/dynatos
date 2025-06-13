//! Effect stack

// Imports
use {
	crate::{Effect, EffectRun},
	core::cell::RefCell,
};

/// Effect stack impl
type EffectStackImpl<F> = RefCell<Vec<Effect<F>>>;

/// Effect stack
#[thread_local]
static EFFECT_STACK: EffectStackImpl<dyn EffectRun> = EffectStackImpl::new(vec![]);

/// Pushes an effect to the stack.
pub fn push<F>(f: Effect<F>)
where
	F: ?Sized + EffectRun,
{
	EFFECT_STACK.borrow_mut().push(f.unsize());
}

/// Pops an effect from the stack
pub fn pop() {
	EFFECT_STACK.borrow_mut().pop().expect("Missing added effect");
}

/// Returns the top effect of the stack
pub fn top() -> Option<Effect> {
	EFFECT_STACK.borrow().last().cloned()
}
