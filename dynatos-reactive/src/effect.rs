//! Effect
//!
//! An effect is a function that is re-run whenever
//! one of it's dependencies changes.

// TODO: Downcasting? It isn't trivial due to the usages of `Rc<Inner<dyn EffectRun>>`,
//       which doesn't allow casting to `Rc<dyn Any>`, required by `Rc::downcast`.

// Modules
mod deps_gatherer;
mod run;
mod suppressed;
mod weak;

// Exports
pub use self::{
	deps_gatherer::EffectDepsGatherer,
	run::{effect_run_impl_inner, EffectRun, EffectRunCtx},
	suppressed::EffectSuppressed,
	weak::WeakEffect,
};

// Imports
use {
	crate::{loc::Loc, WORLD},
	core::{
		cell::Cell,
		fmt,
		hash::{Hash, Hasher},
		marker::Unsize,
		ops::{CoerceUnsized, Deref},
	},
	std::rc::Rc,
};

/// Effect inner
#[doc(hidden)]
pub struct Inner<F: ?Sized> {
	/// Whether this effect is fresh
	fresh: Cell<bool>,

	/// Whether this effect is currently suppressed
	suppressed: Cell<bool>,

	/// Whether we're currently checking dependencies.
	checking_deps: Cell<bool>,

	/// Where this effect was defined
	defined_loc: Loc,

	/// Effect runner
	run: F,
}

// Note: This is necessary to use `Inner` as a receiver
//       for unsizing in `EffectRun`.
impl<F: ?Sized> Deref for Inner<F> {
	type Target = F;

	fn deref(&self) -> &Self::Target {
		&self.run
	}
}

/// Effect
pub struct Effect<F: ?Sized = dyn EffectRun> {
	/// Inner
	pub(crate) inner: Rc<Inner<F>>,
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
			fresh: Cell::new(false),
			suppressed: Cell::new(false),
			checking_deps: Cell::new(false),
			defined_loc: Loc::caller(),
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
	pub(crate) fn defined_loc(&self) -> Loc {
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

	/// Returns a unique identifier to this effect.
	///
	/// Downgrading and cloning the effect will retain the same id
	#[must_use]
	pub fn id(&self) -> usize {
		Rc::as_ptr(&self.inner).addr()
	}

	/// Creates an effect dependency gatherer
	///
	/// While this type lives, all signals used will be gathered as dependencies
	/// for this effect.
	#[must_use]
	pub fn deps_gatherer(&self) -> EffectDepsGatherer<'_, F>
	where
		F: EffectRun + 'static,
	{
		EffectDepsGatherer::new(self)
	}

	/// Gathers dependencies for this effect.
	///
	/// All signals used within `gather` will have this effect as a dependency.
	pub fn gather_dependencies<G, O>(&self, gather: G) -> O
	where
		F: EffectRun + 'static,
		G: FnOnce() -> O,
	{
		let _gatherer = self.deps_gatherer();
		gather()
	}

	/// Runs the effect if stale.
	///
	/// Removes any existing dependencies before running.
	#[track_caller]
	pub fn run(&self)
	where
		F: EffectRun + 'static,
	{
		// If we're checking dependencies, there's a cycle in the dependency graph,
		// so just quit since we're already being executed.
		if self.inner.checking_deps.get() {
			return;
		}

		// Else recursively check dependencies before running
		// TODO: Make it so we don't need to go through all dependencies?
		//       Ideally, we'd check freshness, but when a trigger is executed,
		//       it only marks it's immediate subscribers as stale, instead of
		//       the whole dependency tree. However, we can't make it mark the whole
		//       tree to avoid this check because some subscribers might be marked as
		//       stale when they actually don't need to be rerun (if dependencies change).
		// TODO: Add some logging here to debug why an effect is being run?
		self.inner.checking_deps.set(true);
		WORLD
			.dep_graph
			.with_effect_deps(self.downgrade().unsize(), move |trigger, _| {
				WORLD
					.dep_graph
					.with_trigger_deps(trigger, move |effect, _| _ = effect.try_run());
			});
		self.inner.checking_deps.set(false);

		// If we're suppressed or fresh, we don't need to run.
		if self.is_suppressed() || self.is_fresh() {
			return;
		}

		// Otherwise, force run
		self.force_run();
	}

	/// Runs the effect without checking for freshness.
	///
	/// Removes any existing dependencies before running.
	#[track_caller]
	pub fn force_run(&self)
	where
		F: EffectRun + 'static,
	{
		// Clear the dependencies/subscribers before running
		WORLD.dep_graph.clear_effect(self);

		// Then run it
		let ctx = EffectRunCtx::new();
		let _gatherer = self.deps_gatherer();
		self.inner.run.run(ctx);

		// And set ourselves as fresh
		self.inner.fresh.set(true);
	}

	/// Sets the effect as stale
	pub fn set_stale(&self) {
		self.inner.fresh.set(false);
	}

	/// Returns whether the effect is fresh
	#[must_use]
	pub fn is_fresh(&self) -> bool {
		self.inner.fresh.get()
	}

	/// Returns whether the effect is stale
	#[must_use]
	pub fn is_stale(&self) -> bool {
		!self.is_fresh()
	}

	/// Suppresses this effect.
	pub fn suppress(&self) -> EffectSuppressed<'_, F> {
		EffectSuppressed::new(self)
	}

	/// Returns whether the effect is suppressed
	#[must_use]
	pub fn is_suppressed(&self) -> bool {
		self.inner.suppressed.get()
	}

	/// Formats this effect into `s`
	fn fmt_debug(&self, mut s: fmt::DebugStruct<'_, '_>) -> Result<(), fmt::Error> {
		s.field("id", &self.id());

		s.field("suppressed", &self.inner.suppressed.get());

		s.field_with("defined_loc", |f| fmt::Display::fmt(&self.defined_loc(), f));

		s.finish()
	}

	/// Unsizes this value into an `Effect`.
	// Note: This is necessary for unsizing from `!Sized` to `dyn EffectRun`,
	//       since those coercions only work for `Sized` types.
	// TODO: Once we can unsize from `?Sized` to `dyn EffectRun`,
	//       remove this.
	#[must_use]
	pub fn unsize(self) -> Effect
	where
		F: EffectRun,
	{
		Effect {
			inner: self.inner.unsize_inner(),
		}
	}
}

impl<F1: ?Sized, F2: ?Sized> PartialEq<Effect<F2>> for Effect<F1> {
	fn eq(&self, other: &Effect<F2>) -> bool {
		self.id() == other.id()
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
		self.id().hash(state);
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


/// Returns the current running effect
#[must_use]
pub fn running() -> Option<Effect> {
	WORLD.effect_stack.top()
}

#[cfg(test)]
mod tests;
