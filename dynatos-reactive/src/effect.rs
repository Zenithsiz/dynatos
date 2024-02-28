//! Effect
//!
//! An effect is a function that is re-run whenever
//! one of it's dependencies changes.

// TODO: Downcasting? It isn't trivial due to the usages of `Rc<Inner<dyn Fn()>>`,
//       which doesn't allow casting to `Rc<dyn Any>`, required by `Rc::downcast`.

// Imports
use std::{
	cell::{Cell, RefCell},
	fmt,
	hash::Hash,
	marker::Unsize,
	ops::CoerceUnsized,
	rc::{Rc, Weak},
};

thread_local! {
	/// Effect stack
	static EFFECT_STACK: RefCell<Vec<WeakEffect<dyn Fn()>>> = const { RefCell::new(vec![]) };
}

/// Effect inner
struct Inner<F: ?Sized> {
	/// Whether this effect is currently suppressed
	suppressed: Cell<bool>,

	/// Effect runner
	run: F,
}

impl<F1: ?Sized, F2: ?Sized> CoerceUnsized<Inner<F2>> for Inner<F1> where F1: CoerceUnsized<F2> {}

/// Effect
pub struct Effect<F: ?Sized> {
	/// Inner
	inner: Rc<Inner<F>>,
}

impl<F> Effect<F> {
	/// Creates a new computed effect.
	///
	/// Runs the effect once to gather dependencies.
	pub fn new(run: F) -> Self
	where
		F: Fn() + 'static,
	{
		// Create the effect
		let inner = Inner {
			suppressed: Cell::new(false),
			run,
		};
		let effect = Self { inner: Rc::new(inner) };

		// And run it once to gather dependencies.
		effect.run();

		effect
	}

	/// Tries to create a new effect.
	///
	/// If the effects ends up being inert, returns `None`
	pub fn try_new(run: F) -> Option<Self>
	where
		F: Fn() + 'static,
	{
		let effect = Self::new(run);
		match effect.is_inert() {
			true => None,
			false => Some(effect),
		}
	}
}

impl<F: ?Sized> Effect<F> {
	/// Accesses the inner function
	pub fn inner_fn(&self) -> &F {
		&self.inner.run
	}

	/// Downgrades this effect
	pub fn downgrade(&self) -> WeakEffect<F> {
		WeakEffect {
			inner: Rc::downgrade(&self.inner),
		}
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
	pub fn run(&self)
	where
		F: Fn() + Unsize<dyn Fn()> + 'static,
	{
		// Push the effect, run the closure and pop it
		EFFECT_STACK.with_borrow_mut(|effects| effects.push(self.downgrade()));

		// Then run it, if it's not suppressed
		if !self.inner.suppressed.get() {
			(self.inner.run)();
		}

		// And finally pop the effect from the stack
		EFFECT_STACK
			.with_borrow_mut(|effects| effects.pop())
			.expect("Missing added effect");
	}

	/// Suppresses this effect from running while calling this function
	pub fn suppressed<F2, O>(&self, f: F2) -> O
	where
		F2: FnOnce() -> O,
	{
		// Set the suppress flag and run `f`
		let last_suppressed = self.inner.suppressed.replace(true);
		let output = f();

		// Then restore it
		self.inner.suppressed.set(last_suppressed);

		output
	}
}

impl<F1: ?Sized, F2: ?Sized> PartialEq<Effect<F2>> for Effect<F1> {
	fn eq(&self, other: &Effect<F2>) -> bool {
		self.inner_ptr() == other.inner_ptr()
	}
}

impl<F: ?Sized> Eq for Effect<F> {}

impl<F: ?Sized> Clone for Effect<F> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

impl<F: ?Sized> Hash for Effect<F> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		Rc::as_ptr(&self.inner).hash(state);
	}
}

impl<F: ?Sized> fmt::Debug for Effect<F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Effect").finish_non_exhaustive()
	}
}

impl<T: ?Sized, U: ?Sized> CoerceUnsized<Effect<U>> for Effect<T> where T: Unsize<U> {}


/// Weak effect
///
/// Used to break ownership between a signal and it's subscribers
pub struct WeakEffect<F: ?Sized> {
	/// Inner
	inner: Weak<Inner<F>>,
}

impl<F: ?Sized> WeakEffect<F> {
	/// Upgrades this effect
	pub fn upgrade(&self) -> Option<Effect<F>> {
		self.inner.upgrade().map(|inner| Effect { inner })
	}

	/// Returns the pointer of this effect
	///
	/// This can be used for creating maps based on equality
	pub fn inner_ptr(&self) -> *const () {
		Weak::as_ptr(&self.inner).cast()
	}

	/// Runs this effect, if it exists.
	///
	/// Returns if the effect still existed
	pub fn try_run(&self) -> bool
	where
		F: Fn() + Unsize<dyn Fn()> + 'static,
	{
		// Try to upgrade, else return that it was missing
		let Some(effect) = self.upgrade() else {
			return false;
		};

		effect.run();
		true
	}
}

impl<F1: ?Sized, F2: ?Sized> PartialEq<WeakEffect<F2>> for WeakEffect<F1> {
	fn eq(&self, other: &WeakEffect<F2>) -> bool {
		self.inner_ptr() == other.inner_ptr()
	}
}

impl<F: ?Sized> Eq for WeakEffect<F> {}

impl<F: ?Sized> Clone for WeakEffect<F> {
	fn clone(&self) -> Self {
		Self {
			inner: Weak::clone(&self.inner),
		}
	}
}


impl<F: ?Sized> Hash for WeakEffect<F> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.inner.as_ptr().hash(state);
	}
}

impl<F: ?Sized> fmt::Debug for WeakEffect<F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("WeakEffect").finish_non_exhaustive()
	}
}

impl<T: ?Sized, U: ?Sized> CoerceUnsized<WeakEffect<U>> for WeakEffect<T> where T: Unsize<U> {}

/// Returns the current running effect
pub fn running() -> Option<WeakEffect<dyn Fn()>> {
	EFFECT_STACK.with_borrow(|effects| effects.last().cloned())
}

#[cfg(test)]
mod test {
	// Imports
	use {
		super::{super::effect, *},
		std::cell::OnceCell,
	};

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
				.set(effect::running().expect("Future wasn't running"))
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
				.set(effect::running().expect("Future wasn't running"))
				.expect("Unable to set running effect");

			let effect = Effect::new(move || {
				running_bottom
					.set(effect::running().expect("Future wasn't running"))
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
