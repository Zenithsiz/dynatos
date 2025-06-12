//! Effect
//!
//! An effect is a function that is re-run whenever
//! one of it's dependencies changes.

// TODO: Downcasting? It isn't trivial due to the usages of `Rc<Inner<dyn EffectRun>>`,
//       which doesn't allow casting to `Rc<dyn Any>`, required by `Rc::downcast`.

// Imports
#[cfg(debug_assertions)]
use core::panic::Location;
use {
	crate::{effect_stack, WeakTrigger},
	core::{
		cell::RefCell,
		fmt,
		hash::{Hash, Hasher},
		marker::{PhantomData, Unsize},
		ops::CoerceUnsized,
		ptr,
		sync::atomic::{self, AtomicBool},
	},
	std::{
		collections::HashSet,
		rc::{Rc, Weak},
	},
};

/// Effect inner
pub(crate) struct Inner<F: ?Sized> {
	/// Whether this effect is currently suppressed
	suppressed: AtomicBool,

	#[cfg(debug_assertions)]
	/// Where this effect was defined
	defined_loc: &'static Location<'static>,

	/// All dependencies of this effect
	dependencies: RefCell<HashSet<WeakTrigger>>,

	/// Effect runner
	run: F,
}

/// Effect
pub struct Effect<F: ?Sized = dyn EffectRun> {
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
		F: EffectRun + 'static,
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
			dependencies: RefCell::new(HashSet::new()),
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
		F: EffectRun + 'static,
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
	pub fn inner_fn(&self) -> &F {
		&self.inner.run
	}

	/// Returns where this effect was defined
	#[cfg(debug_assertions)]
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
		F: Unsize<dyn EffectRun> + 'static,
	{
		// Push the effect
		effect_stack::push(self.clone());

		// Then return the gatherer, which will pop the effect from the stack on drop
		EffectDepsGatherer(PhantomData)
	}

	/// Gathers dependencies for this effect.
	///
	/// All signals used within `gather` will have this effect as a dependency.
	pub fn gather_dependencies<G, O>(&self, gather: G) -> O
	where
		F: Unsize<dyn EffectRun> + 'static,
		G: FnOnce() -> O,
	{
		let _gatherer = self.deps_gatherer();
		gather()
	}

	/// Runs the effect.
	///
	/// Removes any existing dependencies before running.
	#[track_caller]
	pub fn run(&self)
	where
		F: EffectRun + Unsize<dyn EffectRun> + 'static,
	{
		// If we're suppressed, don't do anything
		// TODO: Should we clear our dependencies in this case?
		// TODO: Since triggers check if we're suppressed before adding
		//       us to the run queue, should we still need this check here?
		if self.is_suppressed() {
			return;
		}

		// Clear the dependencies before running
		#[expect(clippy::iter_over_hash_type, reason = "We don't care about the order here")]
		for dep in self.inner.dependencies.borrow_mut().drain() {
			let Some(trigger) = dep.upgrade() else { continue };
			trigger.remove_subscriber(self.downgrade());
		}

		// Otherwise, run it
		let ctx = EffectRunCtx { _phantom: PhantomData };
		let _gatherer = self.deps_gatherer();
		self.inner.run.run(ctx);
	}

	/// Suppresses this effect from running while calling this function
	// TODO: Remove this and just add a wrapper around `EffectRun` with the check?
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

	/// Returns whether the effect is suppressed
	#[must_use]
	pub fn is_suppressed(&self) -> bool {
		self.inner.suppressed.load(atomic::Ordering::Acquire)
	}

	/// Adds a dependency to this effect
	pub(crate) fn add_dependency(&self, trigger: WeakTrigger) {
		self.inner.dependencies.borrow_mut().insert(trigger);
	}

	/// Formats this effect into `s`
	fn fmt_debug(&self, mut s: fmt::DebugStruct<'_, '_>) -> Result<(), fmt::Error> {
		s.field("suppressed", &self.inner.suppressed.load(atomic::Ordering::Acquire));

		#[cfg(debug_assertions)]
		s.field_with("defined_loc", |f| fmt::Display::fmt(self.inner.defined_loc, f));

		s.field_with("dependencies", |f| {
			let mut s = f.debug_list();

			let Ok(deps) = self.inner.dependencies.try_borrow() else {
				return s.finish_non_exhaustive();
			};
			#[expect(clippy::iter_over_hash_type, reason = "We don't care about the order")]
			for dep in &*deps {
				let Some(trigger) = dep.upgrade() else {
					s.entry(&"<...>");
					continue;
				};

				s.entry(&trigger);
			}

			s.finish()
		});

		s.finish_non_exhaustive()
	}
}

impl<F1: ?Sized, F2: ?Sized> PartialEq<Effect<F2>> for Effect<F1> {
	fn eq(&self, other: &Effect<F2>) -> bool {
		ptr::eq(self.inner_ptr(), other.inner_ptr())
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
	fn hash<H: Hasher>(&self, state: &mut H) {
		Rc::as_ptr(&self.inner).hash(state);
	}
}

impl<F: ?Sized> fmt::Debug for Effect<F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.fmt_debug(f.debug_struct("Effect"))
	}
}

impl<F1, F2> CoerceUnsized<Effect<F2>> for Effect<F1>
where
	F1: ?Sized + Unsize<F2>,
	F2: ?Sized,
{
}


/// Weak effect
///
/// Used to break ownership between a signal and it's subscribers
pub struct WeakEffect<F: ?Sized = dyn EffectRun> {
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
	#[track_caller]
	#[expect(
		clippy::must_use_candidate,
		reason = "The user may not care whether we actually ran or not"
	)]
	pub fn try_run(&self) -> bool
	where
		F: EffectRun + Unsize<dyn EffectRun> + 'static,
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
		ptr::eq(self.inner_ptr(), other.inner_ptr())
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
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.inner_ptr().hash(state);
	}
}

impl<F: ?Sized> fmt::Debug for WeakEffect<F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("WeakEffect");

		match self.upgrade() {
			Some(effect) => effect.fmt_debug(s),
			None => s.finish_non_exhaustive(),
		}
	}
}

impl<F1, F2> CoerceUnsized<WeakEffect<F2>> for WeakEffect<F1>
where
	F1: ?Sized + Unsize<F2>,
	F2: ?Sized,
{
}

/// Effect dependency gatherer.
///
/// While this type is alive, any signals used will
/// be added as a dependency.
pub struct EffectDepsGatherer<'a>(PhantomData<&'a ()>);

impl Drop for EffectDepsGatherer<'_> {
	fn drop(&mut self) {
		// Pop our effect from the stack
		effect_stack::pop();
	}
}

/// Returns the current running effect
#[must_use]
pub fn running() -> Option<Effect> {
	effect_stack::top()
}

/// Effect run
pub trait EffectRun {
	/// Runs the effect
	#[track_caller]
	fn run(&self, ctx: EffectRunCtx<'_>);
}

/// Effect run context
pub struct EffectRunCtx<'a> {
	_phantom: PhantomData<&'a ()>,
}

impl<F> EffectRun for F
where
	F: Fn(),
{
	fn run(&self, _ctx: EffectRunCtx<'_>) {
		self();
	}
}

#[cfg(test)]
mod tests {
	// Imports
	extern crate test;
	use {
		super::{super::effect, *},
		core::{
			cell::{Cell, OnceCell},
			mem,
		},
		test::Bencher,
	};

	/// Ensures effects are executed
	#[test]
	fn run() {
		#[thread_local]
		static COUNT: Cell<usize> = Cell::new(0);

		assert_eq!(COUNT.get(), 0);
		let effect = Effect::new(|| COUNT.update(|x| x + 1));
		assert_eq!(COUNT.get(), 1);
		effect.run();
		assert_eq!(COUNT.get(), 2);
	}

	/// Ensures the function returned by `Effect::running` is the same as the future being run.
	#[test]
	fn running() {
		#[thread_local]
		static RUNNING: OnceCell<Effect> = OnceCell::new();

		// Create an effect, and save the running effect within it to `RUNNING`.
		let effect = Effect::new(move || {
			RUNNING
				.set(effect::running().expect("Future wasn't running"))
				.expect("Unable to set running effect");
		});

		// Then ensure the running effect is the same as the one created.
		let running = RUNNING.get().expect("Running effect missing");
		assert_eq!(effect, *running);
	}

	/// Ensures the function returned by `Effect::running` is the same as the future being run,
	/// while running stacked futures
	#[test]
	fn running_stacked() {
		#[thread_local]
		static RUNNING_TOP: OnceCell<Effect> = OnceCell::new();

		#[thread_local]
		static RUNNING_BOTTOM: OnceCell<Effect> = OnceCell::new();

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
			let running_bottom = RUNNING_BOTTOM.get().expect("Running effect missing");
			assert_eq!(effect, *running_bottom);
		});

		// Then ensure the top-level running effect is the same as the one created.
		let running_top = RUNNING_TOP.get().expect("Running effect missing");
		assert_eq!(effect, *running_top);

		// And that the bottom-level running effect is already inert
		let running_bottom = RUNNING_BOTTOM.get().expect("Running effect missing");
		assert!(running_bottom.is_inert());
	}

	#[bench]
	fn get_running_100_none(bencher: &mut Bencher) {
		bencher.iter(|| {
			for _ in 0_usize..100 {
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
				for _ in 0_usize..100 {
					let effect = effect::running();
					test::black_box(effect);
				}
			});
		});
	}

	#[bench]
	fn create_10(bencher: &mut Bencher) {
		bencher.iter(|| {
			for _ in 0_usize..10 {
				let effect = Effect::new(move || ());
				test::black_box(&effect);
				mem::forget(effect);
			}
		});
	}
}
