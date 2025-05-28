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
	crate::{world::EffectStack, ReactiveWorld, WeakTrigger},
	core::{
		fmt,
		hash::Hash,
		marker::{PhantomData, Unsize},
		ops::CoerceUnsized,
		sync::atomic::{self, AtomicBool},
	},
	dynatos_world::{IMut, IMutLike, IMutRefMut, Rc, RcLike, Weak, WeakLike, WorldDefault},
	std::collections::HashSet,
};

/// Effect inner
pub(crate) struct Inner<F: ?Sized, W: ReactiveWorld> {
	/// Whether this effect is currently suppressed
	suppressed: AtomicBool,

	#[cfg(debug_assertions)]
	/// Where this effect was defined
	defined_loc: &'static Location<'static>,

	/// All dependencies of this effect
	dependencies: IMut<HashSet<WeakTrigger<W>>, W>,

	/// Lock for gathering dependencies
	gather_deps_lock: IMut<(), W>,

	/// Effect runner
	run: F,
}

/// Effect
pub struct Effect<F: ?Sized, W: ReactiveWorld = WorldDefault> {
	/// Inner
	inner: Rc<Inner<F, W>, W>,
}

impl<F> Effect<F, WorldDefault> {
	/// Creates a new computed effect.
	///
	/// Runs the effect once to gather dependencies.
	#[track_caller]
	pub fn new(run: F) -> Self
	where
		F: EffectRun<WorldDefault> + 'static,
	{
		Self::new_in(run, WorldDefault::default())
	}

	/// Crates a new raw computed effect.
	///
	/// The effect won't be run, and instead you must gather
	/// dependencies manually.
	#[track_caller]
	pub fn new_raw(run: F) -> Self {
		Self::new_raw_in(run, WorldDefault::default())
	}

	/// Tries to create a new effect.
	///
	/// If the effects ends up being inert, returns `None`
	#[track_caller]
	pub fn try_new(run: F) -> Option<Self>
	where
		F: EffectRun<WorldDefault> + 'static,
	{
		Self::try_new_in(run, WorldDefault::default())
	}
}

impl<F, W: ReactiveWorld> Effect<F, W> {
	/// Creates a new computed effect within a world.
	///
	/// Runs the effect once to gather dependencies.
	#[track_caller]
	pub fn new_in(run: F, world: W) -> Self
	where
		F: EffectRun<W> + Unsize<W::F> + 'static,
	{
		// Create the effect
		let effect = Self::new_raw_in(run, world);

		// And run it once to gather dependencies.
		effect.run();

		effect
	}

	/// Crates a new raw computed effect within a world.
	///
	/// The effect won't be run, and instead you must gather
	/// dependencies manually.
	#[track_caller]
	pub fn new_raw_in(run: F, _world: W) -> Self {
		let inner = Inner {
			suppressed: AtomicBool::new(false),
			#[cfg(debug_assertions)]
			defined_loc: Location::caller(),
			dependencies: IMut::<_, W>::new(HashSet::new()),
			gather_deps_lock: IMut::<_, W>::new(()),
			run,
		};

		Self {
			inner: Rc::<_, W>::new(inner),
		}
	}

	/// Tries to create a new effect within a world.
	///
	/// If the effects ends up being inert, returns `None`
	#[track_caller]
	pub fn try_new_in(run: F, world: W) -> Option<Self>
	where
		F: EffectRun<W> + Unsize<W::F> + 'static,
	{
		let effect = Self::new_in(run, world);
		match effect.is_inert() {
			true => None,
			false => Some(effect),
		}
	}
}

impl<F: ?Sized, W: ReactiveWorld> Effect<F, W> {
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
	pub fn downgrade(&self) -> WeakEffect<F, W> {
		WeakEffect {
			inner: Rc::<_, W>::downgrade(&self.inner),
		}
	}

	/// Returns if this effect is inert.
	///
	/// An inert effect is one that will never be updated.
	/// In detail, an effect is inert, if no other [`Effect`]s
	/// or [`WeakEffect`]s exist that point to it.
	#[must_use]
	pub fn is_inert(&self) -> bool {
		Rc::<_, W>::strong_count(&self.inner) == 1 && Rc::<_, W>::weak_count(&self.inner) == 0
	}

	/// Returns the pointer of this effect
	///
	/// This can be used for creating maps based on equality
	#[must_use]
	pub fn inner_ptr(&self) -> *const () {
		Rc::<_, W>::as_ptr(&self.inner).cast()
	}

	/// Creates an effect dependency gatherer
	///
	/// While this type lives, all signals used will be gathered as dependencies
	/// for this effect.
	///
	/// Removes any existing dependencies before returning.
	#[must_use]
	pub fn deps_gatherer(&self) -> EffectDepsGatherer<W>
	where
		F: Unsize<W::F> + 'static,
	{
		// Push the effect
		W::EffectStack::push(self.downgrade());

		// Lock the dependencies gatherer
		let gather_deps_lock = self
			.inner
			.gather_deps_lock
			.try_write()
			.expect("Cannot gather effect dependencies recursively");

		// Clear the dependencies
		let mut deps = self.inner.dependencies.write();
		#[expect(clippy::iter_over_hash_type, reason = "We don't care about the order here")]
		for dep in deps.drain() {
			let Some(trigger) = dep.upgrade() else { continue };
			// TODO: Use `self.downgrade` once we fix issues around the world
			trigger.remove_subscriber(W::EffectStack::top().expect("Just pushed"));
		}

		// Then return the gatherer, which will pop the effect from the stack on drop
		EffectDepsGatherer(gather_deps_lock)
	}

	/// Gathers dependencies for this effect.
	///
	/// All signals used within `gather` will have this effect as a dependency.
	///
	/// Removes any existing dependencies before running.
	pub fn gather_dependencies<G, O>(&self, gather: G) -> O
	where
		F: Unsize<W::F> + 'static,
		G: FnOnce() -> O,
	{
		let _gatherer = self.deps_gatherer();
		gather()
	}

	/// Runs the effect
	#[track_caller]
	pub fn run(&self)
	where
		F: EffectRun<W> + Unsize<W::F> + 'static,
	{
		// If we're suppressed, don't do anything
		// TODO: Should we clear our dependencies in this case?
		// TODO: Since triggers check if we're suppressed before adding
		//       us to the run queue, should we still need this check here?
		if self.is_suppressed() {
			return;
		}

		// Otherwise, run it
		let ctx = EffectRunCtx {
			// TODO: Not have to clone the effect each time we run?
			effect:   W::unsize_effect(self.clone()),
			_phantom: PhantomData,
		};
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
	pub fn is_suppressed(&self) -> bool {
		self.inner.suppressed.load(atomic::Ordering::Acquire)
	}

	/// Formats this effect into `s`
	fn fmt_debug(&self, mut s: fmt::DebugStruct<'_, '_>) -> Result<(), fmt::Error> {
		s.field("suppressed", &self.inner.suppressed.load(atomic::Ordering::Acquire));

		#[cfg(debug_assertions)]
		s.field_with("defined_loc", |f| fmt::Display::fmt(self.inner.defined_loc, f));

		s.field_with("dependencies", |f| {
			let mut s = f.debug_list();

			let Some(deps) = self.inner.dependencies.try_read() else {
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

impl<F1: ?Sized, F2: ?Sized, W: ReactiveWorld> PartialEq<Effect<F2, W>> for Effect<F1, W> {
	fn eq(&self, other: &Effect<F2, W>) -> bool {
		core::ptr::eq(self.inner_ptr(), other.inner_ptr())
	}
}

impl<F: ?Sized, W: ReactiveWorld> Eq for Effect<F, W> {}

impl<F: ?Sized, W: ReactiveWorld> Clone for Effect<F, W> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::<_, W>::clone(&self.inner),
		}
	}
}

impl<F: ?Sized, W: ReactiveWorld> Hash for Effect<F, W> {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		Rc::<_, W>::as_ptr(&self.inner).hash(state);
	}
}

impl<F: ?Sized, W: ReactiveWorld> fmt::Debug for Effect<F, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.fmt_debug(f.debug_struct("Effect"))
	}
}

impl<F1, F2, W> CoerceUnsized<Effect<F2, W>> for Effect<F1, W>
where
	F1: ?Sized + Unsize<F2>,
	F2: ?Sized,
	W: ReactiveWorld,
	Rc<Inner<F1, W>, W>: CoerceUnsized<Rc<Inner<F2, W>, W>>,
{
}


/// Weak effect
///
/// Used to break ownership between a signal and it's subscribers
pub struct WeakEffect<F: ?Sized, W: ReactiveWorld = WorldDefault> {
	/// Inner
	inner: Weak<Inner<F, W>, W>,
}

impl<F: ?Sized, W: ReactiveWorld> WeakEffect<F, W> {
	/// Upgrades this effect
	#[must_use]
	pub fn upgrade(&self) -> Option<Effect<F, W>> {
		self.inner.upgrade().map(|inner| Effect { inner })
	}

	/// Returns the pointer of this effect
	///
	/// This can be used for creating maps based on equality
	#[must_use]
	pub fn inner_ptr(&self) -> *const () {
		Weak::<_, W>::as_ptr(&self.inner).cast()
	}

	/// Runs this effect, if it exists.
	///
	/// Returns if the effect still existed
	#[track_caller]
	pub fn try_run(&self) -> bool
	where
		F: EffectRun<W> + Unsize<W::F> + 'static,
	{
		// Try to upgrade, else return that it was missing
		let Some(effect) = self.upgrade() else {
			return false;
		};

		effect.run();
		true
	}

	/// Adds a dependency to this effect
	pub(crate) fn add_dependency(&self, trigger: WeakTrigger<W>) {
		let Some(effect) = self.upgrade() else { return };
		effect.inner.dependencies.write().insert(trigger);
	}
}

impl<F1: ?Sized, F2: ?Sized, W: ReactiveWorld> PartialEq<WeakEffect<F2, W>> for WeakEffect<F1, W> {
	fn eq(&self, other: &WeakEffect<F2, W>) -> bool {
		core::ptr::eq(self.inner_ptr(), other.inner_ptr())
	}
}

impl<F: ?Sized, W: ReactiveWorld> Eq for WeakEffect<F, W> {}

impl<F: ?Sized, W: ReactiveWorld> Clone for WeakEffect<F, W> {
	fn clone(&self) -> Self {
		Self {
			inner: Weak::<_, W>::clone(&self.inner),
		}
	}
}


impl<F: ?Sized, W: ReactiveWorld> Hash for WeakEffect<F, W> {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.inner_ptr().hash(state);
	}
}

impl<F: ?Sized, W: ReactiveWorld> fmt::Debug for WeakEffect<F, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("WeakEffect");

		match self.upgrade() {
			Some(effect) => effect.fmt_debug(s),
			None => s.finish_non_exhaustive(),
		}
	}
}

impl<F1, F2, W> CoerceUnsized<WeakEffect<F2, W>> for WeakEffect<F1, W>
where
	F1: ?Sized + Unsize<F2>,
	F2: ?Sized,
	W: ReactiveWorld,
	Weak<Inner<F1, W>, W>: CoerceUnsized<Weak<Inner<F2, W>, W>>,
{
}

/// Effect dependency gatherer.
///
/// While this type is alive, any signals used will
/// be added as a dependency.
pub struct EffectDepsGatherer<'a, W: ReactiveWorld = WorldDefault>(IMutRefMut<'a, (), W>);

impl<W: ReactiveWorld> Drop for EffectDepsGatherer<'_, W> {
	fn drop(&mut self) {
		// Pop our effect from the stack
		W::EffectStack::pop();
	}
}

/// Returns the current running effect
// TODO: Move this elsewhere
#[must_use]
pub fn running<W: ReactiveWorld>() -> Option<WeakEffect<W::F, W>> {
	<W>::EffectStack::top()
}

/// Effect run
pub trait EffectRun<W: ReactiveWorld = WorldDefault> {
	/// Runs the effect
	fn run(&self, ctx: EffectRunCtx<'_, W>);
}

/// Effect run context
pub struct EffectRunCtx<'a, W: ReactiveWorld> {
	/// Effect
	effect: Effect<W::F, W>,

	_phantom: PhantomData<&'a ()>,
}

impl<'a, W: ReactiveWorld> EffectRunCtx<'a, W> {
	/// Returns the effect running this
	pub fn effect(&self) -> Effect<W::F, W> {
		self.effect.clone()
	}
}

impl<F, W> EffectRun<W> for F
where
	F: Fn(),
	W: ReactiveWorld,
{
	fn run(&self, _ctx: EffectRunCtx<'_, W>) {
		self();
	}
}

#[cfg(test)]
mod test {
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
		static RUNNING: OnceCell<WeakEffect<dyn EffectRun, WorldDefault>> = OnceCell::new();

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
		static RUNNING_TOP: OnceCell<WeakEffect<dyn EffectRun>> = OnceCell::new();

		#[thread_local]
		static RUNNING_BOTTOM: OnceCell<WeakEffect<dyn EffectRun>> = OnceCell::new();

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
				let effect = effect::running::<WorldDefault>();
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
					let effect = effect::running::<WorldDefault>();
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
