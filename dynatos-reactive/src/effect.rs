//! Effect
//!
//! An effect is a function that is re-run whenever
//! one of it's dependencies changes.

// TODO: Downcasting? It isn't trivial due to the usages of `Rc<Inner<dyn Fn()>>`,
//       which doesn't allow casting to `Rc<dyn Any>`, required by `Rc::downcast`.

// Imports
#[cfg(debug_assertions)]
use core::panic::Location;
use {
	core::{
		cell::RefCell,
		fmt,
		hash::Hash,
		marker::Unsize,
		ops::CoerceUnsized,
		sync::atomic::{self, AtomicBool},
	},
	dynatos_reactive_sync::{Rc, SyncBounds, Weak},
};

/// Effect stack
#[thread_local]
static EFFECT_STACK: RefCell<Vec<WeakEffect<dyn Fn() + SyncBounds>>> = RefCell::new(vec![]);

/// Effect inner
struct Inner<F: ?Sized> {
	/// Whether this effect is currently suppressed
	suppressed: AtomicBool,

	#[cfg(debug_assertions)]
	/// Where this effect was defined
	defined_loc: &'static Location<'static>,

	/// Effect runner
	run: F,
}

impl<F1, F2> CoerceUnsized<Inner<F2>> for Inner<F1>
where
	F1: ?Sized + CoerceUnsized<F2>,
	F2: ?Sized,
{
}

/// Effect
pub struct Effect<F: ?Sized> {
	/// Inner
	inner: Rc<Inner<F>>,
}

impl<F> Effect<F> {
	/// Creates a new computed effect.
	///
	/// Runs the effect once to gather dependencies.
	#[track_caller]
	pub fn new(run: F) -> Self
	where
		F: Fn() + 'static + SyncBounds,
	{
		// Create the effect
		let effect = Self::new_raw(run);

		// And run it once to gather dependencies.
		effect.run();

		effect
	}

	/// Crates a new raw computed effect.
	///
	/// The effect won't be run, and instead you must gather
	/// dependencies manually.
	#[track_caller]
	pub fn new_raw(run: F) -> Self {
		let inner = Inner {
			suppressed: AtomicBool::new(false),
			#[cfg(debug_assertions)]
			defined_loc: Location::caller(),
			run,
		};

		Self { inner: Rc::new(inner) }
	}

	/// Tries to create a new effect.
	///
	/// If the effects ends up being inert, returns `None`
	#[track_caller]
	pub fn try_new(run: F) -> Option<Self>
	where
		F: Fn() + 'static + SyncBounds,
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
	#[must_use]
	#[expect(clippy::missing_const_for_fn, reason = "False positive")]
	pub fn inner_fn(&self) -> &F {
		&self.inner.run
	}

	/// Returns where this effect was defined
	#[cfg(debug_assertions)]
	#[expect(clippy::missing_const_for_fn, reason = "False positive")]
	pub(crate) fn defined_loc(&self) -> &'static Location<'static> {
		self.inner.defined_loc
	}

	/// Downgrades this effect
	#[must_use]
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
	#[must_use]
	pub fn is_inert(&self) -> bool {
		Rc::strong_count(&self.inner) == 1 && Rc::weak_count(&self.inner) == 0
	}

	/// Returns the pointer of this effect
	///
	/// This can be used for creating maps based on equality
	#[must_use]
	pub fn inner_ptr(&self) -> *const () {
		Rc::as_ptr(&self.inner).cast()
	}

	/// Creates an effect dependency gatherer
	///
	/// While this type lives, all signals used will be gathered as dependencies
	/// for this effect.
	#[must_use]
	pub fn deps_gatherer(&self) -> EffectDepsGatherer
	where
		F: Unsize<dyn Fn() + SyncBounds> + 'static,
	{
		// Push the effect, run the closure and pop it
		EFFECT_STACK.borrow_mut().push(self.downgrade());

		// Then return the gatherer, which will pop the effect from the stack on drop
		EffectDepsGatherer(())
	}

	/// Gathers dependencies for this effect.
	///
	/// All signals used within `gather` will have this effect as a dependency.
	pub fn gather_dependencies<G, O>(&self, gather: G) -> O
	where
		F: Unsize<dyn Fn() + SyncBounds> + 'static,
		G: FnOnce() -> O,
	{
		let _gatherer = self.deps_gatherer();
		gather()
	}

	/// Runs the effect
	pub fn run(&self)
	where
		F: Fn() + Unsize<dyn Fn() + SyncBounds> + 'static,
	{
		// If we're suppressed, don't do anything
		if self.inner.suppressed.load(atomic::Ordering::Acquire) {
			return;
		}

		// Otherwise, run it
		self.gather_dependencies(move || (self.inner.run)());
	}

	/// Suppresses this effect from running while calling this function
	pub fn suppressed<F2, O>(&self, f: F2) -> O
	where
		F2: FnOnce() -> O,
	{
		// Set the suppress flag and run `f`
		let last_suppressed = self.inner.suppressed.swap(true, atomic::Ordering::AcqRel);
		let output = f();

		// Then restore it
		self.inner.suppressed.store(last_suppressed, atomic::Ordering::Release);

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
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		Rc::as_ptr(&self.inner).hash(state);
	}
}

impl<F: ?Sized> fmt::Debug for Effect<F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Effect").finish_non_exhaustive()
	}
}

impl<T, U> CoerceUnsized<Effect<U>> for Effect<T>
where
	T: ?Sized + Unsize<U>,
	U: ?Sized,
{
}


/// Weak effect
///
/// Used to break ownership between a signal and it's subscribers
pub struct WeakEffect<F: ?Sized> {
	/// Inner
	inner: Weak<Inner<F>>,
}

impl<F: ?Sized> WeakEffect<F> {
	/// Upgrades this effect
	#[must_use]
	pub fn upgrade(&self) -> Option<Effect<F>> {
		self.inner.upgrade().map(|inner| Effect { inner })
	}

	/// Returns the pointer of this effect
	///
	/// This can be used for creating maps based on equality
	#[must_use]
	pub fn inner_ptr(&self) -> *const () {
		Weak::as_ptr(&self.inner).cast()
	}

	/// Runs this effect, if it exists.
	///
	/// Returns if the effect still existed
	#[expect(
		clippy::must_use_candidate,
		reason = "The user may just want to run the effect, without checking if it exists"
	)]
	pub fn try_run(&self) -> bool
	where
		F: Fn() + Unsize<dyn Fn() + SyncBounds> + 'static,
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
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.inner_ptr().hash(state);
	}
}

impl<F: ?Sized> fmt::Debug for WeakEffect<F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("WeakEffect").finish_non_exhaustive()
	}
}

impl<T, U> CoerceUnsized<WeakEffect<U>> for WeakEffect<T>
where
	T: ?Sized + Unsize<U>,
	U: ?Sized,
{
}

/// Effect dependency gatherer.
///
/// While this type is alive, any signals used will
/// be added as a dependency.
pub struct EffectDepsGatherer(());

impl Drop for EffectDepsGatherer {
	fn drop(&mut self) {
		// Pop our effect from the stack
		EFFECT_STACK.borrow_mut().pop().expect("Missing added effect");
	}
}

/// Returns the current running effect
pub fn running() -> Option<WeakEffect<dyn Fn() + SyncBounds>> {
	EFFECT_STACK.borrow().last().cloned()
}

#[cfg(test)]
mod test {
	// Imports
	extern crate test;
	use {
		super::{super::effect, *},
		core::{cell::OnceCell, mem},
		test::Bencher,
	};

	/// Ensures the function returned by `Effect::running` is the same as the future being run.
	#[test]
	fn running() {
		#[thread_local]
		static RUNNING: OnceCell<WeakEffect<dyn Fn()>> = OnceCell::new();

		// Create an effect, and save the running effect within it to `RUNNING`.
		let effect = Effect::new(move || {
			RUNNING
				.set(effect::running().expect("Future wasn't running"))
				.expect("Unable to set running effect");
		});

		// Then ensure the running effect is the same as the one created.
		let running = RUNNING
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
		#[thread_local]
		static RUNNING_TOP: OnceCell<WeakEffect<dyn Fn()>> = OnceCell::new();

		#[thread_local]
		static RUNNING_BOTTOM: OnceCell<WeakEffect<dyn Fn()>> = OnceCell::new();

		// Create 2 stacked effects, saving the running within each to `running1` and `running2`.
		// `running1` contains the top-level effect, while `running2` contains the inner one.
		let effect = Effect::new(move || {
			RUNNING_TOP
				.set(effect::running().expect("Future wasn't running"))
				.expect("Unable to set running effect");

			let effect = Effect::new(move || {
				RUNNING_BOTTOM
					.set(effect::running().expect("Future wasn't running"))
					.expect("Unable to set running effect");
			});

			// Then ensure the bottom-level running effect is the same as the one created.
			let running_bottom = RUNNING_BOTTOM
				.get()
				.expect("Running effect missing")
				.upgrade()
				.expect("Running effect was dropped");
			assert_eq!(effect, running_bottom);
		});

		// Then ensure the top-level running effect is the same as the one created.
		let running_top = RUNNING_TOP
			.get()
			.expect("Running effect missing")
			.upgrade()
			.expect("Running effect was dropped");
		assert_eq!(effect, running_top);

		// And that the bottom-level running effect was already dropped
		let running_bottom = RUNNING_BOTTOM.get().expect("Running effect missing").upgrade();
		assert_eq!(running_bottom, None);
	}

	#[bench]
	fn get_running_100_none(bencher: &mut Bencher) {
		bencher.iter(|| {
			for _ in 0..100 {
				let effect = effect::running();
				test::black_box(effect);
			}
		});
	}

	#[bench]
	fn get_running_100_some(bencher: &mut Bencher) {
		let effect = Effect::new_raw(move || ());

		effect.gather_dependencies(|| {
			bencher.iter(|| {
				for _ in 0..100 {
					let effect = effect::running();
					test::black_box(effect);
				}
			});
		});
	}

	#[bench]
	fn create_10(bencher: &mut Bencher) {
		bencher.iter(|| {
			for _ in 0..10 {
				let effect = Effect::new(move || ());
				test::black_box(&effect);
				mem::forget(effect);
			}
		});
	}
}
