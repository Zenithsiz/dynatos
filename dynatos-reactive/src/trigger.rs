//! Trigger
//!
//! A reactivity primitive that allows re-running
//! any subscribers.

// Imports
use {
	crate::{effect, Effect, WeakEffect},
	core::{
		fmt,
		marker::Unsize,
		mem::{self, ManuallyDrop},
		task,
	},
	dynatos_reactive_sync::{IMut, IMutExt, Rc, SyncBounds, Weak},
	std::collections::{hash_map, HashMap},
};
#[cfg(debug_assertions)]
use {
	core::{iter, panic::Location},
	std::collections::HashSet,
};

/// Subscribers
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub struct Subscriber {
	/// Effect
	effect: WeakEffect<dyn Fn() + SyncBounds>,
}

/// Subscriber info
#[derive(Clone, Debug)]
struct SubscriberInfo {
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
struct Inner {
	/// Subscribers
	#[cfg_attr(
		not(debug_assertions),
		expect(
			clippy::zero_sized_map_values,
			reason = "It isn't zero-sized with `debug_assertions`"
		)
	)]
	subscribers: IMut<HashMap<Subscriber, SubscriberInfo>>,

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
		let inner = Inner {
			#[cfg_attr(
				not(debug_assertions),
				expect(
					clippy::zero_sized_map_values,
					reason = "It isn't zero-sized with `debug_assertions`"
				)
			)]
			subscribers: IMut::new(HashMap::new()),
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
		if let Some(effect) = effect::running() {
			self.add_subscriber(effect);
		}
	}

	/// Adds a subscriber to this trigger.
	///
	/// Returns if the subscriber already existed.
	#[track_caller]
	fn add_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		let mut subscribers = self.inner.subscribers.imut_write();
		match subscribers.entry(subscriber.into_subscriber()) {
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
	fn _remove_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		Self::remove_subscriber_inner(&self.inner, subscriber)
	}

	/// Inner function for [`Self::remove_subscriber`]
	#[track_caller]
	fn remove_subscriber_inner<S: IntoSubscriber>(inner: &Inner, subscriber: S) -> bool {
		let mut subscribers = inner.subscribers.imut_write();
		subscribers.remove(&subscriber.into_subscriber()).is_some()
	}

	/// Triggers this trigger.
	///
	/// Re-runs all subscribers.
	pub fn trigger(&self) {
		Self::trigger_inner(&self.inner);
	}

	/// Inner function for [`Self::trigger`]
	fn trigger_inner(inner: &Inner) {
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
			.imut_write()
			.iter()
			.map(|(subscriber, info)| (subscriber.clone(), info.clone()))
			.collect::<Vec<_>>();
		for (subscriber, info) in subscribers {
			let Some(effect) = subscriber.effect.upgrade() else {
				Self::remove_subscriber_inner(inner, subscriber);
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

	/// Creates a `RawWaker`.
	///
	/// By default, this can only be passed to [`task::LocalWaker`],
	/// however, if the `sync` feature is enabled, this can be passed
	/// to [`task::Waker`] as well.
	#[must_use]
	pub fn into_raw_waker(self) -> task::RawWaker {
		fn into_inner(ptr: *const ()) -> Rc<Inner> {
			// SAFETY: All callers of this function pass in the correct arguments.
			unsafe { Rc::from_raw(ptr.cast()) }
		}

		fn with_inner<F, O>(ptr: *const (), f: F) -> O
		where
			F: FnOnce(&Rc<Inner>) -> O,
		{
			// Note: The inner Rc is never dropped, so this won't double drop
			let rc = ManuallyDrop::new(into_inner(ptr));
			f(&rc)
		}

		fn from_inner(inner: Rc<Inner>) -> task::RawWaker {
			task::RawWaker::new(Rc::into_raw(inner).cast(), &VTABLE)
		}

		fn clone(ptr: *const ()) -> task::RawWaker {
			from_inner(with_inner(ptr, Rc::clone))
		}

		fn wake(ptr: *const ()) {
			wake_by_ref(ptr);
			drop(ptr);
		}

		fn wake_by_ref(ptr: *const ()) {
			with_inner(ptr, |inner| Trigger::trigger_inner(inner));
		}

		fn drop(ptr: *const ()) {
			mem::drop(into_inner(ptr));
		}

		const VTABLE: task::RawWakerVTable = task::RawWakerVTable::new(clone, wake, wake_by_ref, drop);

		from_inner(self.inner)
	}
}

impl Default for Trigger {
	fn default() -> Self {
		Self::new()
	}
}

impl Clone for Trigger {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

impl fmt::Debug for Trigger {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Trigger").finish_non_exhaustive()
	}
}

/// Weak trigger
pub struct WeakTrigger {
	/// Inner
	inner: Weak<Inner>,
}

impl WeakTrigger {
	/// Upgrades this weak trigger
	#[must_use]
	pub fn upgrade(&self) -> Option<Trigger> {
		let inner = self.inner.upgrade()?;
		Some(Trigger { inner })
	}
}

impl Clone for WeakTrigger {
	fn clone(&self) -> Self {
		Self {
			inner: Weak::clone(&self.inner),
		}
	}
}

impl fmt::Debug for WeakTrigger {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("WeakTrigger").finish_non_exhaustive()
	}
}

/// Types that may be converted into a subscriber
pub trait IntoSubscriber {
	/// Converts this type into a weak effect.
	fn into_subscriber(self) -> Subscriber;
}

impl IntoSubscriber for Subscriber {
	fn into_subscriber(self) -> Subscriber {
		self
	}
}

#[duplicate::duplicate_item(
	T effect_value;
	[ Effect ] [ self.downgrade() ];
	[ &'_ Effect ] [ self.downgrade() ];
	[ WeakEffect ] [ self ];
)]
impl<F> IntoSubscriber for T<F>
where
	F: ?Sized + Fn() + Unsize<dyn Fn() + SyncBounds> + 'static,
{
	#[track_caller]
	fn into_subscriber(self) -> Subscriber {
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
		let triggers = core::array::from_fn::<_, 100, _>(|_| Trigger::new());
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
