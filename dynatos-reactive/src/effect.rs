//! Effect
//!
//! An effect is a function that is re-run whenever
//! one of it's dependencies changes.

// Imports
use std::{
	cell::RefCell,
	fmt,
	hash::Hash,
	rc::{Rc, Weak},
};

thread_local! {
	/// Effect stack
	static EFFECT_STACK: RefCell<Vec<Effect>> = RefCell::new(vec![]);
}

/// Effect inner
struct Inner {
	/// Effect runner
	run: Option<Box<dyn Fn()>>,
}

/// Effect
#[derive(Clone)]
pub struct Effect {
	/// Inner
	inner: Rc<RefCell<Inner>>,
}

impl Effect {
	/// Creates a new computed effect.
	///
	/// Runs the effect once to gather dependencies.
	pub fn new<F>(run: F) -> Self
	where
		F: Fn() + 'static,
	{
		// Create the effect
		let inner = Inner {
			run: Some(Box::new(run)),
		};
		let effect = Self {
			inner: Rc::new(RefCell::new(inner)),
		};

		// And run it once to gather dependencies.
		effect.run();

		effect
	}

	/// Tries to create a new effect.
	///
	/// If the effects ends up being inert, returns `None`
	pub fn try_new<F>(run: F) -> Option<Self>
	where
		F: Fn() + 'static,
	{
		let effect = Self::new(run);
		match effect.is_inert() {
			true => None,
			false => Some(effect),
		}
	}

	/// Downgrades this effect
	pub fn downgrade(&self) -> WeakEffect {
		WeakEffect {
			inner: Rc::downgrade(&self.inner),
		}
	}

	/// Returns the current running effect
	pub fn running() -> Option<Self> {
		EFFECT_STACK.with_borrow(|effects| effects.last().cloned())
	}

	/// Returns if this effect is inert.
	///
	/// An inert effect is one that will never be updated.
	/// In detail, an effect is inert, if no other [`Effect`]s
	/// or [`WeakEffect`]s exist that point to it.
	pub fn is_inert(&self) -> bool {
		Rc::strong_count(&self.inner) == 1 && Rc::weak_count(&self.inner) == 0
	}

	/// Runs the effect
	pub fn run(&self) {
		// Push the effect, run the closure and pop it
		EFFECT_STACK.with_borrow_mut(|effects| effects.push(self.clone()));

		// Then run it
		let inner = self.inner.borrow();
		if let Some(run) = inner.run.as_ref() {
			run();
		}

		// And finally pop the effect from the stack
		EFFECT_STACK
			.with_borrow_mut(|effects| effects.pop())
			.expect("Missing added effect");
	}

	/// Suppresses this effect from running while calling this function
	pub fn suppressed<F, O>(&self, f: F) -> O
	where
		F: FnOnce() -> O,
	{
		// Remove the run function and run `f`
		let run = self.inner.borrow_mut().run.take();
		let output = f();

		// Then put it back
		self.inner.borrow_mut().run = run;

		output
	}
}

impl PartialEq for Effect {
	fn eq(&self, other: &Self) -> bool {
		Rc::ptr_eq(&self.inner, &other.inner)
	}
}

impl Eq for Effect {}

impl Hash for Effect {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.inner.as_ptr().hash(state);
	}
}

impl fmt::Debug for Effect {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Effect").finish_non_exhaustive()
	}
}


/// Weak effect
///
/// Used to break ownership between a signal and it's subscribers
#[derive(Clone)]
pub struct WeakEffect {
	/// Inner
	inner: Weak<RefCell<Inner>>,
}

impl WeakEffect {
	/// Upgrades this effect
	pub fn upgrade(&self) -> Option<Effect> {
		self.inner.upgrade().map(|inner| Effect { inner })
	}

	/// Runs this effect, if it exists.
	///
	/// Returns if the effect still existed
	pub fn try_run(&self) -> bool {
		// Try to upgrade, else return that it was missing
		let Some(effect) = self.upgrade() else {
			return false;
		};

		effect.run();
		true
	}
}

impl PartialEq for WeakEffect {
	fn eq(&self, other: &Self) -> bool {
		Weak::ptr_eq(&self.inner, &other.inner)
	}
}

impl Eq for WeakEffect {}

impl Hash for WeakEffect {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.inner.as_ptr().hash(state);
	}
}

impl fmt::Debug for WeakEffect {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("WeakEffect").finish_non_exhaustive()
	}
}
