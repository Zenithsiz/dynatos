//! Effect stack

// Imports
use {
	crate::{Effect, EffectRun},
	core::cell::RefCell,
};

/// Effect stack
#[derive(Debug)]
pub struct EffectStack {
	/// Stack
	stack: RefCell<Vec<Effect>>,
}

impl EffectStack {
	/// Creates a new, empty, effect stack
	#[must_use]
	pub const fn new() -> Self {
		Self {
			stack: RefCell::new(vec![]),
		}
	}

	/// Pushes an effect to the stack.
	pub fn push<F>(&self, f: Effect<F>)
	where
		F: ?Sized + EffectRun,
	{
		self.stack.borrow_mut().push(f.unsize());
	}

	/// Pops an effect from the stack
	pub fn pop(&self) {
		self.stack.borrow_mut().pop().expect("Missing added effect");
	}

	/// Returns the top effect of the stack
	pub fn top(&self) -> Option<Effect> {
		self.stack.borrow().last().cloned()
	}
}

impl Default for EffectStack {
	fn default() -> Self {
		Self::new()
	}
}
