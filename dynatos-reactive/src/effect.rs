//! Effect
//!
//! An effect is a function that is re-run whenever
//! one of it's dependencies changes.

// Imports
use std::{
	cell::RefCell,
	fmt,
	hash::Hash,
	mem,
	rc::{Rc, Weak},
};

thread_local! {
	/// Effect stack
	static EFFECT_STACK: RefCell<Vec<WeakEffect>> = const { RefCell::new(vec![]) };
}

/// Effect inner
struct Inner {
	/// Whether to ignore running the effect
	ignore: bool,

	/// Effect runner
	run: Box<dyn Fn()>,
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
			ignore: false,
			run:    Box::new(run),
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
	pub fn running() -> Option<WeakEffect> {
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

	/// Returns the pointer of this effect
	///
	/// This can be used for creating maps based on equality
	pub fn inner_ptr(&self) -> *const () {
		Rc::as_ptr(&self.inner).cast()
	}

	/// Runs the effect
	pub fn run(&self) {
		// Push the effect, run the closure and pop it
		EFFECT_STACK.with_borrow_mut(|effects| effects.push(self.downgrade()));

		// Then run it, if it's not ignored
		let inner = self.inner.borrow();
		if !inner.ignore {
			(inner.run)();
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
		// Set the ignore flag and run `f`
		let last = mem::replace(&mut self.inner.borrow_mut().ignore, true);
		let output = f();

		// Then restore it
		self.inner.borrow_mut().ignore = last;

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

#[cfg(test)]
mod test {
	// Imports
	use {super::*, std::cell::OnceCell};

	/// Leaks a value and returns a `&'static T`
	///
	/// This is useful because `&'static T` is `Copy`,
	/// so we don't need to worry about cloning `Rc`s to
	/// pass variables to effects.
	fn leaked<T>(value: T) -> &'static T {
		Box::leak(Box::new(value))
	}

	/// Ensures the function returned by `Effect::running` is the same as the future being run.
	#[test]
	fn running() {
		// Create an effect, and save the running effect within it to `running`.
		let running = self::leaked(OnceCell::new());
		let effect = Effect::new(move || {
			running
				.set(Effect::running().expect("Future wasn't running"))
				.expect("Unable to set running effect");
		});

		// Then ensure the running effect is the same as the one created.
		let running = running
			.get()
			.expect("Running effect missing")
			.upgrade()
			.expect("Running effect was dropped");
		assert_eq!(effect, running);
	}

	/// Ensures the function returned by `Effect::running` is the same as the future being run,
	/// while running stacked futures
	#[test]
	fn running_stacked() {
		// Create 2 stacked effects, saving the running within each to `running1` and `running2`.
		// `running1` contains the top-level effect, while `running2` contains the inner one.
		let running_top = self::leaked(OnceCell::new());
		let running_bottom = self::leaked(OnceCell::new());
		let effect = Effect::new(move || {
			running_top
				.set(Effect::running().expect("Future wasn't running"))
				.expect("Unable to set running effect");

			let effect = Effect::new(move || {
				running_bottom
					.set(Effect::running().expect("Future wasn't running"))
					.expect("Unable to set running effect");
			});

			// Then ensure the bottom-level running effect is the same as the one created.
			let running_bottom = running_bottom
				.get()
				.expect("Running effect missing")
				.upgrade()
				.expect("Running effect was dropped");
			assert_eq!(effect, running_bottom);
		});

		// Then ensure the top-level running effect is the same as the one created.
		let running_top = running_top
			.get()
			.expect("Running effect missing")
			.upgrade()
			.expect("Running effect was dropped");
		assert_eq!(effect, running_top);

		// And that the bottom-level running effect was already dropped
		let running_bottom = running_bottom.get().expect("Running effect missing").upgrade();
		assert_eq!(running_bottom, None);
	}
}
