//! Trigger
//!
//! A reactivity primitive that allows re-running
//! any subscribers.

// Imports
use {
	crate::{effect, world, Effect, ReactiveWorld, WeakEffect},
	core::{
		fmt,
		hash::{Hash, Hasher},
		ops::CoerceUnsized,
	},
	dynatos_world::{IMut, IMutLike, Rc, RcLike, Weak, WeakLike, WorldDefault},
	std::collections::{hash_map, HashMap},
};
#[cfg(debug_assertions)]
use {
	core::{iter, panic::Location},
	std::collections::HashSet,
};

/// Subscribers
#[derive(Debug)]
pub struct Subscriber<W: ReactiveWorld> {
	/// Effect
	effect: WeakEffect<world::F<W>, W>,
}

impl<W: ReactiveWorld> Clone for Subscriber<W> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
		}
	}
}

impl<W: ReactiveWorld> PartialEq for Subscriber<W> {
	fn eq(&self, other: &Self) -> bool {
		self.effect == other.effect
	}
}

impl<W: ReactiveWorld> Eq for Subscriber<W> {}

impl<W: ReactiveWorld> Hash for Subscriber<W> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.effect.hash(state);
	}
}

/// Subscriber info
#[derive(Clone, Debug)]
pub(crate) struct SubscriberInfo {
	#[cfg(debug_assertions)]
	/// Where this subscriber was defined
	defined_locs: HashSet<&'static Location<'static>>,
}

impl SubscriberInfo {
	/// Creates new subscriber info.
	#[track_caller]
	#[cfg_attr(
		not(debug_assertions),
		expect(
			clippy::missing_const_for_fn,
			reason = "It can't be a `const fn` with `debug_assertions`"
		)
	)]
	pub fn new() -> Self {
		Self {
			#[cfg(debug_assertions)]
			defined_locs:                          iter::once(Location::caller()).collect(),
		}
	}

	/// Updates this subscriber info
	#[track_caller]
	#[cfg_attr(
		not(debug_assertions),
		expect(clippy::unused_self, reason = "We use it with `debug_assertions`")
	)]
	pub fn update(&mut self) {
		#[cfg(debug_assertions)]
		self.defined_locs.insert(Location::caller());
	}
}

/// Trigger inner
struct Inner<W: ReactiveWorld> {
	/// Subscribers
	#[cfg_attr(
		not(debug_assertions),
		expect(
			clippy::zero_sized_map_values,
			reason = "It isn't zero-sized with `debug_assertions`"
		)
	)]
	subscribers: IMut<HashMap<Subscriber<W>, SubscriberInfo>, W>,

	#[cfg(debug_assertions)]
	/// Where this trigger was defined
	defined_loc: &'static Location<'static>,
}

/// Trigger
pub struct Trigger<W: ReactiveWorld = WorldDefault> {
	/// Inner
	inner: Rc<Inner<W>, W>,
}

impl Trigger<WorldDefault> {
	/// Creates a new trigger
	#[must_use]
	#[track_caller]
	pub fn new() -> Self {
		Self::new_in(WorldDefault::default())
	}
}

impl<W: ReactiveWorld> Trigger<W> {
	/// Creates a new trigger in a world
	#[must_use]
	#[track_caller]
	pub fn new_in(_world: W) -> Self {
		let inner = Inner {
			#[cfg_attr(
				not(debug_assertions),
				expect(
					clippy::zero_sized_map_values,
					reason = "It isn't zero-sized with `debug_assertions`"
				)
			)]
			subscribers: IMut::<_, W>::new(HashMap::new()),
			#[cfg(debug_assertions)]
			defined_loc: Location::caller(),
		};
		Self {
			inner: Rc::<_, W>::new(inner),
		}
	}

	/// Returns where this effect was defined
	#[cfg(debug_assertions)]
	pub(crate) fn defined_loc(&self) -> &'static Location<'static> {
		self.inner.defined_loc
	}

	/// Downgrades this trigger
	#[must_use]
	pub fn downgrade(&self) -> WeakTrigger<W> {
		WeakTrigger {
			inner: Rc::<_, W>::downgrade(&self.inner),
		}
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
		if let Some(effect) = effect::running::<W>() {
			#[cfg(debug_assertions)]
			effect.add_dependency(self.downgrade());
			self.add_subscriber(effect);
		}
	}

	/// Adds a subscriber to this trigger.
	///
	/// Returns if the subscriber already existed.
	#[track_caller]
	fn add_subscriber<S: IntoSubscriber<W>>(&self, subscriber: S) -> bool {
		let mut subscribers = self.inner.subscribers.write();
		match (*subscribers).entry(subscriber.into_subscriber()) {
			hash_map::Entry::Occupied(mut entry) => {
				entry.get_mut().update();
				true
			},
			hash_map::Entry::Vacant(entry) => {
				entry.insert(SubscriberInfo::new());
				false
			},
		}
	}

	/// Removes a subscriber from this trigger.
	///
	/// Returns if the subscriber existed
	#[track_caller]
	fn _remove_subscriber<S: IntoSubscriber<W>>(&self, subscriber: S) -> bool {
		Self::remove_subscriber_inner(&self.inner, subscriber)
	}

	/// Inner function for [`Self::remove_subscriber`]
	#[track_caller]
	fn remove_subscriber_inner<S: IntoSubscriber<W>>(inner: &Inner<W>, subscriber: S) -> bool {
		let mut subscribers = inner.subscribers.write();
		subscribers.remove(&subscriber.into_subscriber()).is_some()
	}

	/// Triggers this trigger.
	///
	/// Re-runs all subscribers.
	pub fn trigger(&self) {
		Self::trigger_inner(&self.inner);
	}

	/// Inner function for [`Self::trigger`]
	fn trigger_inner(inner: &Inner<W>) {
		// Run all subscribers, and remove any empty ones
		// Note: Since running the subscriber might add subscribers, we can't keep
		//       the inner borrow active, so we gather all dependencies before-hand.
		//       However, we can remove subscribers in between running effects, so we
		//       don't need to wait for that.
		// TODO: Have a 2nd field `to_add_subscribers` where subscribers are added if
		//       the main field is locked, and after this loop move any subscribers from
		//       it to the main field?
		let subscribers = inner
			.subscribers
			.write()
			.iter()
			.map(|(subscriber, info)| (subscriber.clone(), info.clone()))
			.collect::<Vec<_>>();
		for (subscriber, info) in subscribers {
			let Some(effect) = subscriber.effect.upgrade() else {
				Self::remove_subscriber_inner(inner, subscriber.effect);
				continue;
			};

			#[cfg(debug_assertions)]
			{
				use itertools::Itertools;
				tracing::trace!(
					effect_loc=%effect.defined_loc(),
					subscriber_locs=%info.defined_locs.iter().copied().map(Location::to_string).join(";"),
					trigger_loc=%inner.defined_loc,
					"Running effect due to trigger"
				);
			};
			#[cfg(not(debug_assertions))]
			let _: SubscriberInfo = info;

			effect.run();
		}
	}

	/// Formats this trigger into `s`
	fn fmt_debug(&self, mut s: fmt::DebugStruct<'_, '_>) -> Result<(), fmt::Error> {
		#[cfg(debug_assertions)]
		s.field_with("defined_loc", |f| fmt::Display::fmt(self.inner.defined_loc, f));

		s.finish_non_exhaustive()
	}
}

impl<W: ReactiveWorld + Default> Default for Trigger<W> {
	fn default() -> Self {
		Self::new_in(W::default())
	}
}

impl<W: ReactiveWorld> PartialEq for Trigger<W> {
	fn eq(&self, other: &Self) -> bool {
		core::ptr::eq(Rc::<_, W>::as_ptr(&self.inner), Rc::<_, W>::as_ptr(&other.inner))
	}
}

impl<W: ReactiveWorld> Eq for Trigger<W> {}


impl<W: ReactiveWorld> Clone for Trigger<W> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::<_, W>::clone(&self.inner),
		}
	}
}

impl<W: ReactiveWorld> Hash for Trigger<W> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		Rc::<_, W>::as_ptr(&self.inner).hash(state);
	}
}

impl<W: ReactiveWorld> fmt::Debug for Trigger<W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.fmt_debug(f.debug_struct("Trigger"))
	}
}

/// Weak trigger
pub struct WeakTrigger<W: ReactiveWorld> {
	/// Inner
	inner: Weak<Inner<W>, W>,
}

impl<W: ReactiveWorld> WeakTrigger<W> {
	/// Upgrades this weak trigger
	#[must_use]
	pub fn upgrade(&self) -> Option<Trigger<W>> {
		let inner = self.inner.upgrade()?;
		Some(Trigger { inner })
	}
}

impl<W: ReactiveWorld> PartialEq for WeakTrigger<W> {
	fn eq(&self, other: &Self) -> bool {
		core::ptr::eq(Weak::<_, W>::as_ptr(&self.inner), Weak::<_, W>::as_ptr(&other.inner))
	}
}

impl<W: ReactiveWorld> Eq for WeakTrigger<W> {}

impl<W: ReactiveWorld> Clone for WeakTrigger<W> {
	fn clone(&self) -> Self {
		Self {
			inner: Weak::<_, W>::clone(&self.inner),
		}
	}
}

impl<W: ReactiveWorld> Hash for WeakTrigger<W> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		Weak::<_, W>::as_ptr(&self.inner).hash(state);
	}
}

impl<W: ReactiveWorld> fmt::Debug for WeakTrigger<W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("WeakTrigger");

		match self.upgrade() {
			Some(trigger) => trigger.fmt_debug(s),
			None => s.finish_non_exhaustive(),
		}
	}
}

/// Types that may be converted into a subscriber
pub trait IntoSubscriber<W: ReactiveWorld> {
	/// Converts this type into a weak effect.
	fn into_subscriber(self) -> Subscriber<W>;
}

impl<W: ReactiveWorld> IntoSubscriber<W> for Subscriber<W> {
	fn into_subscriber(self) -> Self {
		self
	}
}

#[expect(clippy::allow_attributes, reason = "Only applicable to one of the branches")]
#[allow(clippy::use_self, reason = "Only applicable in one of the branches")]
#[duplicate::duplicate_item(
	T effect_value;
	[ Effect ] [ self.downgrade() ];
	[ &'_ Effect ] [ self.downgrade() ];
	[ WeakEffect ] [ self ];
)]
impl<F, W> IntoSubscriber<W> for T<F, W>
where
	F: ?Sized + core::marker::Unsize<world::F<W>>,
	W: ReactiveWorld,
	WeakEffect<F, W>: CoerceUnsized<WeakEffect<world::F<W>, W>>,
{
	#[track_caller]
	fn into_subscriber(self) -> Subscriber<W> {
		Subscriber { effect: effect_value }
	}
}

#[cfg(test)]
mod test {
	// Imports
	extern crate test;
	use {
		super::*,
		core::{cell::Cell, mem},
		test::Bencher,
	};

	#[test]
	fn basic() {
		/// Counts the number of times the effect was run
		#[thread_local]
		static TRIGGERS: Cell<usize> = Cell::new(0);

		// Create the effect and reset the flag
		let effect = Effect::new(move || TRIGGERS.set(TRIGGERS.get() + 1));

		// Then create the trigger, and ensure it wasn't triggered
		// by just creating it and adding the subscriber
		let trigger = Trigger::new();
		trigger.add_subscriber(&effect);
		assert_eq!(TRIGGERS.get(), 1, "Trigger was triggered early");

		// Then trigger and ensure it was triggered
		trigger.trigger();
		assert_eq!(TRIGGERS.get(), 2, "Trigger was not triggered");

		// Then add the subscriber again and ensure the effect isn't run twice
		trigger.add_subscriber(&effect);
		trigger.trigger();
		assert_eq!(TRIGGERS.get(), 3, "Trigger ran effect multiple times");

		// Finally drop the effect and try again
		mem::drop(effect);
		trigger.trigger();
		assert_eq!(TRIGGERS.get(), 3, "Trigger was triggered after effect was dropped");
	}

	#[bench]
	fn clone_100(bencher: &mut Bencher) {
		let triggers = core::array::from_fn::<Trigger, 100, _>(|_| Trigger::new());
		bencher.iter(|| {
			for trigger in &triggers {
				let trigger = test::black_box(trigger.clone());
				mem::forget(trigger);
			}
		});
	}

	/// Benches triggering a trigger with `N` no-op effects.
	fn trigger_noop_n<const N: usize>(bencher: &mut Bencher) {
		let trigger: Trigger = Trigger::new();
		let effects = core::array::from_fn::<_, N, _>(|_| Effect::new(|| ()));
		for effect in &effects {
			trigger.add_subscriber(effect);
		}

		bencher.iter(|| {
			trigger.trigger();
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
