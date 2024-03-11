//! Trigger
//!
//! A reactivity primitive that allows re-running
//! any subscribers.

// Imports
use {
	crate::{IntoSubscriber, WeakEffect},
	core::{cell::RefCell, fmt},
	std::{
		collections::HashSet,
		rc::{Rc, Weak},
	},
};

/// Trigger inner
struct Inner {
	/// Subscribers
	subscribers: RefCell<HashSet<WeakEffect<dyn Fn()>>>,
}

/// Trigger
pub struct Trigger {
	/// Inner
	inner: Rc<Inner>,
}

impl Trigger {
	/// Creates a new trigger
	#[must_use]
	pub fn new() -> Self {
		let inner = Inner {
			subscribers: RefCell::new(HashSet::new()),
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

	/// Adds a subscriber to this trigger.
	///
	/// Returns if the subscriber already existed.
	pub fn add_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		let mut subscribers = self.inner.subscribers.borrow_mut();
		let new_effect = subscribers.insert(subscriber.into_subscriber());
		!new_effect
	}

	/// Removes a subscriber from this trigger.
	///
	/// Returns if the subscriber existed
	pub fn remove_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		let mut subscribers = self.inner.subscribers.borrow_mut();
		subscribers.remove(&subscriber.into_subscriber())
	}

	/// Triggers this trigger.
	///
	/// Re-runs all subscribers.
	pub fn trigger(&self) {
		// Run all subscribers, and remove any empty ones
		// Note: Since running the subscriber might add subscribers, we can't keep
		//       the inner borrow active, so we gather all dependencies before-hand.
		//       However, we can remove subscribers in between running effects, so we
		//       don't need to wait for that.
		// TODO: Have a 2nd field `to_add_subscribers` where subscribers are added if
		//       the main field is locked, and after this loop move any subscribers from
		//       it to the main field?
		let subscribers = self.inner.subscribers.borrow().iter().cloned().collect::<Vec<_>>();
		for subscriber in subscribers {
			if !subscriber.try_run() {
				self.remove_subscriber(subscriber);
			}
		}
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

#[cfg(test)]
mod test {
	// Imports
	extern crate test;
	use {
		super::*,
		crate::Effect,
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
