//! Trigger
//!
//! A reactivity primitive that allows re-running
//! any subscribers.

// Imports
use {
	crate::{IntoSubscriber, WeakEffect},
	std::{cell::RefCell, collections::HashSet, fmt, rc::Rc},
};

/// Trigger inner
struct Inner {
	/// Subscribers
	subscribers: HashSet<WeakEffect<dyn Fn()>>,
}

/// Trigger
pub struct Trigger {
	/// Inner
	inner: Rc<RefCell<Inner>>,
}

impl Trigger {
	/// Creates a new trigger
	pub fn new() -> Self {
		let inner = Inner {
			subscribers: HashSet::new(),
		};
		Self {
			inner: Rc::new(RefCell::new(inner)),
		}
	}

	/// Adds a subscriber to this trigger.
	///
	/// Returns if the subscriber already existed.
	pub fn add_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		let mut inner = self.inner.borrow_mut();
		let new_effect = inner.subscribers.insert(subscriber.into_subscriber());
		!new_effect
	}

	/// Removes a subscriber from this trigger.
	///
	/// Returns if the subscriber existed
	pub fn remove_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		let mut inner = self.inner.borrow_mut();
		inner.subscribers.remove(&subscriber.into_subscriber())
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
		let subscribers = self.inner.borrow().subscribers.iter().cloned().collect::<Vec<_>>();
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

#[cfg(test)]
mod test {
	// Imports
	use {
		super::*,
		crate::Effect,
		std::{cell::Cell, mem},
	};

	#[test]
	fn basic() {
		thread_local! {
			/// Counts the number of times the effect was run
			static TRIGGERS: Cell<usize> = const { Cell::new(0) };
		}

		// Create the effect and reset the flag
		let effect = Effect::new(move || TRIGGERS.set(TRIGGERS.get() + 1));

		// Then create the trigger, and ensure it wasn't triggered
		// by just creating it and adding the subscriber
		let trigger = Trigger::new();
		trigger.add_subscriber(effect.downgrade() as WeakEffect<dyn Fn()>);
		assert_eq!(TRIGGERS.get(), 1, "Trigger was triggered early");

		// Then trigger and ensure it was triggered
		trigger.trigger();
		assert_eq!(TRIGGERS.get(), 2, "Trigger was not triggered");

		// Then add the subscriber again and ensure the effect isn't run twice
		trigger.add_subscriber(effect.downgrade() as WeakEffect<dyn Fn()>);
		trigger.trigger();
		assert_eq!(TRIGGERS.get(), 3, "Trigger ran effect multiple times");

		// Finally drop the effect and try again
		mem::drop(effect);
		trigger.trigger();
		assert_eq!(TRIGGERS.get(), 3, "Trigger was triggered after effect was dropped");
	}
}
