//! Trigger
//!
//! A reactivity primitive that allows re-running
//! any subscribers.

// Imports
use {
	crate::{effect, run_queue, Effect, EffectRun, WeakEffect},
	core::{
		cell::{LazyCell, RefCell},
		fmt,
		hash::{Hash, Hasher},
		ptr,
	},
	std::{
		collections::{hash_map, HashMap},
		rc::{Rc, Weak},
	},
};
#[cfg(debug_assertions)]
use {
	core::{iter, panic::Location},
	std::collections::HashSet,
};

/// Subscriber / Dependency info
// TODO: Make cloning this cheap by wrapping it in an `Arc` or something.
#[derive(Clone, Debug)]
pub struct TriggerEffectInfo {
	#[cfg(debug_assertions)]
	/// Where this subscriber / dependency was defined
	defined_locs: HashSet<&'static Location<'static>>,
}

#[cfg_attr(
	not(debug_assertions),
	expect(
		clippy::missing_const_for_fn,
		reason = "It can't be a `const fn` with `debug_assertions`"
	)
)]
impl TriggerEffectInfo {
	/// Creates new info.
	#[track_caller]
	#[must_use]
	pub fn new() -> Self {
		Self::new_with(
			#[cfg(debug_assertions)]
			Location::caller(),
		)
	}

	/// Creates new info with the caller location
	#[must_use]
	pub fn new_with(#[cfg(debug_assertions)] caller_loc: &'static Location<'static>) -> Self {
		Self {
			#[cfg(debug_assertions)]
			defined_locs:                          iter::once(caller_loc).collect(),
		}
	}

	/// Updates this info
	#[track_caller]
	pub fn update(&mut self) {
		self.update_with(
			#[cfg(debug_assertions)]
			Location::caller(),
		);
	}

	/// Updates this info with the caller location
	pub fn update_with(&mut self, #[cfg(debug_assertions)] caller_loc: &'static Location<'static>) {
		#[cfg(debug_assertions)]
		self.defined_locs.insert(caller_loc);
	}
}

impl Default for TriggerEffectInfo {
	fn default() -> Self {
		Self::new()
	}
}

/// Trigger inner
#[cfg_attr(
	not(debug_assertions),
	expect(
		clippy::zero_sized_map_values,
		reason = "They aren't zero-sized with `debug_assertions`"
	)
)]
struct Inner {
	/// Dependencies
	dependencies: RefCell<HashMap<WeakEffect, TriggerEffectInfo>>,

	/// Subscribers
	subscribers: RefCell<HashMap<WeakEffect, TriggerEffectInfo>>,

	#[cfg(debug_assertions)]
	/// Where this trigger was defined
	defined_loc: &'static Location<'static>,
}

/// Trigger
pub struct Trigger {
	/// Inner
	inner: Rc<Inner>,
}

impl Trigger {
	/// Creates a new trigger
	#[must_use]
	#[track_caller]
	pub fn new() -> Self {
		#[cfg_attr(
			not(debug_assertions),
			expect(
				clippy::zero_sized_map_values,
				reason = "They aren't zero-sized with `debug_assertions`"
			)
		)]
		let inner = Inner {
			dependencies: RefCell::new(HashMap::new()),
			subscribers: RefCell::new(HashMap::new()),
			#[cfg(debug_assertions)]
			defined_loc: Location::caller(),
		};
		Self { inner: Rc::new(inner) }
	}

	/// Downgrades this trigger
	#[must_use]
	pub fn downgrade(&self) -> WeakTrigger {
		WeakTrigger {
			inner: Rc::downgrade(&self.inner),
		}
	}

	/// Returns where this effect was defined
	#[cfg(debug_assertions)]
	pub(crate) fn defined_loc(&self) -> &'static Location<'static> {
		self.inner.defined_loc
	}

	/// Returns the pointer of this effect
	///
	/// This can be used for creating maps based on equality
	#[must_use]
	pub fn inner_ptr(&self) -> *const () {
		Rc::as_ptr(&self.inner).cast()
	}

	/// Gathers all effects depending on this trigger.
	///
	/// When triggering this trigger, all effects active during this gathering
	/// will be re-run.
	///
	/// You can gather multiple times without removing the previous gathered
	/// effects. Previous effects will only be removed when they are dropped.
	// TODO: Should we remove all existing subscribers before gathering them?
	#[track_caller]
	pub fn gather_subscribers(&self) {
		match effect::running() {
			Some(effect) => {
				#[cfg(debug_assertions)]
				tracing::trace!(
					"Adding effect dependency\nEffect  : {}\nTrigger : {}\nGathered: {}",
					effect.defined_loc(),
					self.defined_loc(),
					Location::caller(),
				);

				effect.add_dependency(self.downgrade());
				self.add_subscriber(effect);
			},

			// TODO: Add some way to turn off this warning at a global
			//       scale, with something like
			//       `fn without_warning(f: impl FnOnce() -> O) -> O`
			#[cfg(debug_assertions)]
			None => tracing::warn!(
				trigger=?self,
				location=%Location::caller(),
				"No effect is being run when trigger was accessed. \
				\nThis typically means that you're accessing reactive \
				signals outside of an effect, which means the code won't \
				be re-run when the signal changes. If this is intention, \
				try to use one of the `_raw` methods that don't gather \
				subscribers to make it intentional"
			),

			#[cfg(not(debug_assertions))]
			None => (),
		}
	}

	/// Adds a subscriber to this trigger.
	///
	/// Returns if the subscriber already existed.
	#[track_caller]
	fn add_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		let mut subscribers = self.inner.subscribers.borrow_mut();
		match (*subscribers).entry(subscriber.into_subscriber()) {
			hash_map::Entry::Occupied(mut entry) => {
				entry.get_mut().update();
				true
			},
			hash_map::Entry::Vacant(entry) => {
				entry.insert(TriggerEffectInfo::new());
				false
			},
		}
	}

	/// Adds a dependency to this trigger.
	///
	/// Returns if the dependency already existed.
	fn add_dependency(
		&self,
		dependency: WeakEffect,
		#[cfg(debug_assertions)] caller_loc: &'static Location<'static>,
	) -> bool {
		let mut dependencies = self.inner.dependencies.borrow_mut();
		match (*dependencies).entry(dependency) {
			hash_map::Entry::Occupied(mut entry) => {
				entry.get_mut().update_with(
					#[cfg(debug_assertions)]
					caller_loc,
				);
				true
			},
			hash_map::Entry::Vacant(entry) => {
				entry.insert(TriggerEffectInfo::new_with(
					#[cfg(debug_assertions)]
					caller_loc,
				));
				false
			},
		}
	}

	/// Removes a subscriber from this trigger.
	///
	/// Returns if the subscriber existed
	#[track_caller]
	pub(crate) fn remove_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		let mut subscribers = self.inner.subscribers.borrow_mut();
		subscribers.remove(&subscriber.into_subscriber()).is_some()
	}

	/// Removes a dependency from this trigger.
	///
	/// Returns if the dependency existed
	#[track_caller]
	pub(crate) fn remove_dependency(&self, dependency: &WeakEffect) -> bool {
		let mut dependencies = self.inner.dependencies.borrow_mut();
		dependencies.remove(dependency).is_some()
	}

	/// Executes this trigger.
	///
	/// Adds all subscribers to the run queue, and once the returned
	/// executor is dropped, and there are no other executors alive,
	/// all queues effects are run.
	#[track_caller]
	#[expect(
		clippy::must_use_candidate,
		reason = "The user can just immediately drop the value to execute if they don't care"
	)]
	pub fn exec(&self) -> TriggerExec {
		self.exec_inner(
			#[cfg(debug_assertions)]
			Location::caller(),
		)
	}

	/// Creates an execution for a no-op trigger.
	///
	/// This is useful to ensure that another trigger
	/// doesn't execute the run queue and just appends to
	/// it instead.
	pub fn exec_noop() -> TriggerExec {
		/// No-op trigger
		#[thread_local]
		static NOOP_TRIGGER: LazyCell<Trigger> = LazyCell::new(Trigger::new);

		NOOP_TRIGGER.exec()
	}

	/// Inner function for [`Self::exec`]
	pub(crate) fn exec_inner(&self, #[cfg(debug_assertions)] caller_loc: &'static Location<'static>) -> TriggerExec {
		// If there's a running effect, register it as our dependency
		if let Some(effect) = effect::running() {
			#[cfg(debug_assertions)]
			tracing::trace!(
				"Adding effect subscriber\nEffect  : {}\nTrigger : {}\nExecuted: {}",
				effect.defined_loc(),
				self.defined_loc(),
				caller_loc,
			);

			effect.add_subscriber(self.downgrade());
			self.add_dependency(
				effect.downgrade(),
				#[cfg(debug_assertions)]
				caller_loc,
			);
		}

		let subscribers = self.inner.subscribers.borrow();

		// Increase the ref count
		run_queue::inc_ref();

		// Then all all of our subscribers
		// TODO: Should we care about the order? Randomizing it is probably good, since
		//       it'll bring to the surface weird bugs or performance dependent on effect run order.
		#[expect(clippy::iter_over_hash_type, reason = "We don't care about which order they go in")]
		for (subscriber, info) in &*subscribers {
			// If the effect doesn't exist anymore, remove it
			let Some(effect) = subscriber.upgrade() else {
				continue;
			};

			// Skip suppressed effects
			if effect.is_suppressed() {
				continue;
			}

			// TODO: Should the run queue use strong effects?
			run_queue::push(subscriber.clone(), info.clone());
		}

		TriggerExec {
			#[cfg(debug_assertions)]
			trigger_defined_loc:                          self.inner.defined_loc,
			#[cfg(debug_assertions)]
			exec_defined_loc:                             caller_loc,
		}
	}

	/// Formats this trigger into `s`
	#[cfg_attr(
		not(debug_assertions),
		expect(clippy::unused_self, reason = "We use it in with `debug_assertions`")
	)]
	fn fmt_debug(&self, mut s: fmt::DebugStruct<'_, '_>) -> Result<(), fmt::Error> {
		s.field_with("inner", |f| fmt::Pointer::fmt(&self.inner_ptr(), f));

		#[cfg(debug_assertions)]
		s.field_with("defined_loc", |f| fmt::Display::fmt(self.inner.defined_loc, f));

		s.field_with("dependencies", |f| {
			Self::fmt_debug_effect_map(f, &self.inner.dependencies)
		});
		s.field_with("subscribers", |f| {
			Self::fmt_debug_effect_map(f, &self.inner.subscribers)
		});

		s.finish()
	}

	/// Formats an effect hashmap (dependencies / subscribers) into `f`.
	fn fmt_debug_effect_map(
		f: &mut fmt::Formatter<'_>,
		map: &RefCell<HashMap<WeakEffect, TriggerEffectInfo>>,
	) -> Result<(), fmt::Error> {
		#[cfg(debug_assertions)]
		let mut s = f.debug_map();
		#[cfg(not(debug_assertions))]
		let mut s = f.debug_list();

		let Ok(deps) = map.try_borrow() else {
			return s.finish_non_exhaustive();
		};
		#[expect(clippy::iter_over_hash_type, reason = "We don't care about the order")]
		for (dep, info) in &*deps {
			#[cfg(not(debug_assertions))]
			let _ = info;

			let Some(effect) = dep.upgrade() else {
				#[cfg(debug_assertions)]
				s.entry(&"<...>", info);

				#[cfg(not(debug_assertions))]
				s.entry_with(|f| {
					fmt::Pointer::fmt(&dep.inner_ptr(), f)?;
					f.write_str(" (dead)")
				});

				continue;
			};

			#[cfg(debug_assertions)]
			s.key_with(|f| fmt::Display::fmt(&effect.defined_loc(), f))
				.value_with(|f| Self::fmt_debug_effect_info(f, info));

			#[cfg(not(debug_assertions))]
			s.entry_with(|f| fmt::Pointer::fmt(&effect.inner_ptr(), f));
		}

		s.finish()
	}

	/// Formats an effect info into `f`.
	#[cfg(debug_assertions)]
	fn fmt_debug_effect_info(f: &mut fmt::Formatter<'_>, info: &TriggerEffectInfo) -> Result<(), fmt::Error> {
		let mut s = f.debug_struct("Info");

		s.field_with("defined_locs", |f| {
			let mut s = f.debug_list();

			#[expect(clippy::iter_over_hash_type, reason = "We don't care about order for debugging")]
			for &loc in &info.defined_locs {
				s.entry_with(|f| fmt::Display::fmt(loc, f));
			}

			s.finish()
		});

		s.finish()
	}
}

impl Default for Trigger {
	fn default() -> Self {
		Self::new()
	}
}

impl PartialEq for Trigger {
	fn eq(&self, other: &Self) -> bool {
		ptr::eq(Rc::as_ptr(&self.inner), Rc::as_ptr(&other.inner))
	}
}

impl Eq for Trigger {}


impl Clone for Trigger {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

impl Hash for Trigger {
	fn hash<H: Hasher>(&self, state: &mut H) {
		Rc::as_ptr(&self.inner).hash(state);
	}
}

impl fmt::Debug for Trigger {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.fmt_debug(f.debug_struct("Trigger"))
	}
}

/// Weak trigger
pub struct WeakTrigger {
	/// Inner
	inner: Weak<Inner>,
}

impl WeakTrigger {
	/// Creates an empty weak trigger
	#[must_use]
	pub const fn new() -> Self {
		Self { inner: Weak::new() }
	}

	/// Upgrades this weak trigger
	#[must_use]
	pub fn upgrade(&self) -> Option<Trigger> {
		let inner = self.inner.upgrade()?;
		Some(Trigger { inner })
	}
}

impl Default for WeakTrigger {
	fn default() -> Self {
		Self::new()
	}
}

impl PartialEq for WeakTrigger {
	fn eq(&self, other: &Self) -> bool {
		ptr::eq(Weak::as_ptr(&self.inner), Weak::as_ptr(&other.inner))
	}
}

impl Eq for WeakTrigger {}

impl Clone for WeakTrigger {
	fn clone(&self) -> Self {
		Self {
			inner: Weak::clone(&self.inner),
		}
	}
}

impl Hash for WeakTrigger {
	fn hash<H: Hasher>(&self, state: &mut H) {
		Weak::as_ptr(&self.inner).hash(state);
	}
}

impl fmt::Debug for WeakTrigger {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("WeakTrigger");

		match self.upgrade() {
			Some(trigger) => trigger.fmt_debug(s),
			None => s.finish_non_exhaustive(),
		}
	}
}

/// Types that may be converted into a subscriber
pub trait IntoSubscriber {
	/// Converts this type into a weak effect.
	#[track_caller]
	fn into_subscriber(self) -> WeakEffect;
}

#[expect(clippy::allow_attributes, reason = "Only applicable to one of the branches")]
#[allow(clippy::use_self, reason = "Only applicable in one of the branches")]
#[duplicate::duplicate_item(
	T effect_value;
	[ Effect ] [ self.downgrade() ];
	[ &'_ Effect ] [ self.downgrade() ];
	[ WeakEffect ] [ self ];
)]
impl<F> IntoSubscriber for T<F>
where
	F: ?Sized + EffectRun,
{
	fn into_subscriber(self) -> WeakEffect {
		effect_value.unsize()
	}
}

/// Trigger executor
pub struct TriggerExec {
	/// Trigger defined location
	#[cfg(debug_assertions)]
	trigger_defined_loc: &'static Location<'static>,

	/// Execution defined location
	#[cfg(debug_assertions)]
	exec_defined_loc: &'static Location<'static>,
}

impl Drop for TriggerExec {
	fn drop(&mut self) {
		// Decrease the reference count, and if we weren't the last, quit
		let Some(_exec_guard) = run_queue::dec_ref() else {
			return;
		};

		// If we were the last, keep popping effects and running them until
		// the run queue is empty
		while let Some((subscriber, info)) = run_queue::pop() {
			let Some(effect) = subscriber.upgrade() else {
				continue;
			};

			#[cfg(debug_assertions)]
			tracing::trace!(
				"Running effect due to trigger\nEffect   : {}\nSubscribers: {}\nTrigger  : {}\nExecution: {}",
				effect.defined_loc(),
				match info.defined_locs.is_empty() {
					true => "[]".to_owned(),
					#[expect(clippy::format_collect, reason = "TODO")]
					false => info
						.defined_locs
						.iter()
						.copied()
						.map(|loc| format!("\n         - {loc}"))
						.collect::<String>(),
				},
				self.trigger_defined_loc,
				self.exec_defined_loc,
			);


			#[cfg(not(debug_assertions))]
			let _: TriggerEffectInfo = info;

			effect.run();
		}
	}
}

#[cfg(test)]
mod tests {
	// Imports
	extern crate test;
	use {
		super::*,
		core::{array, cell::Cell, mem},
		test::Bencher,
		zutil_cloned::cloned,
	};

	#[test]
	fn basic() {
		/// Counts the number of times the effect was run
		#[thread_local]
		static TRIGGERS: Cell<usize> = Cell::new(0);

		let trigger = Trigger::new();
		#[cloned(trigger)]
		let effect = Effect::new(move || {
			trigger.gather_subscribers();
			TRIGGERS.set(TRIGGERS.get() + 1);
		});

		assert_eq!(TRIGGERS.get(), 1, "Trigger was triggered early");

		// Then trigger and ensure it was triggered
		trigger.exec();
		assert_eq!(TRIGGERS.get(), 2, "Trigger was not triggered");

		// Finally drop the effect and try again
		mem::drop(effect);
		trigger.exec();
		assert_eq!(TRIGGERS.get(), 2, "Trigger was triggered after effect was dropped");
	}

	#[test]
	fn exec_multiple() {
		/// Counts the number of times the effect was run
		#[thread_local]
		static TRIGGERS: Cell<usize> = Cell::new(0);

		let trigger = Trigger::new();
		#[cloned(trigger)]
		let _effect = Effect::new(move || {
			trigger.gather_subscribers();
			TRIGGERS.set(TRIGGERS.get() + 1);
		});

		let exec0 = trigger.exec();
		assert_eq!(TRIGGERS.get(), 1, "Trigger was triggered when executing");
		let exec1 = trigger.exec();

		drop(exec1);
		assert_eq!(
			TRIGGERS.get(),
			1,
			"Trigger was triggered when dropping a single executor"
		);

		drop(exec0);
		assert_eq!(
			TRIGGERS.get(),
			2,
			"Trigger wasn't triggered when dropping last executor"
		);
	}

	#[test]
	fn exec_multiple_same_effect() {
		/// Counts the number of times the effect was run
		#[thread_local]
		static TRIGGERS: Cell<usize> = Cell::new(0);

		let trigger0 = Trigger::new();
		let trigger1 = Trigger::new();
		#[cloned(trigger0, trigger1)]
		let _effect = Effect::new(move || {
			trigger0.gather_subscribers();
			trigger1.gather_subscribers();
			TRIGGERS.set(TRIGGERS.get() + 1);
		});

		let exec0 = trigger0.exec();
		let exec1 = trigger1.exec();

		drop((exec0, exec1));

		assert_eq!(TRIGGERS.get(), 2, "Effect was run multiple times in same run queue");

		trigger0.exec();

		assert_eq!(
			TRIGGERS.get(),
			3,
			"Effect wasn't run even when no other executors existed"
		);
	}

	#[bench]
	fn clone_100(bencher: &mut Bencher) {
		let triggers = array::from_fn::<Trigger, 100, _>(|_| Trigger::new());
		bencher.iter(|| {
			for trigger in &triggers {
				let trigger = test::black_box(trigger.clone());
				mem::forget(trigger);
			}
		});
	}

	/// Benches triggering a trigger with `N` no-op effects.
	fn trigger_noop_n<const N: usize>(bencher: &mut Bencher) {
		let trigger = Trigger::new();
		let _effects = array::from_fn::<_, N, _>(|_| {
			Effect::new(
				#[cloned(trigger)]
				move || trigger.gather_subscribers(),
			)
		});

		bencher.iter(|| {
			trigger.exec();
		});
	}

	#[bench]
	fn trigger_empty(bencher: &mut Bencher) {
		self::trigger_noop_n::<0>(bencher);
	}

	#[bench]
	fn trigger_noop(bencher: &mut Bencher) {
		self::trigger_noop_n::<1>(bencher);
	}

	#[bench]
	fn trigger_noop_10(bencher: &mut Bencher) {
		self::trigger_noop_n::<10>(bencher);
	}

	#[bench]
	fn trigger_noop_100(bencher: &mut Bencher) {
		self::trigger_noop_n::<100>(bencher);
	}

	#[bench]
	fn trigger_noop_1000(bencher: &mut Bencher) {
		self::trigger_noop_n::<1000>(bencher);
	}
}
