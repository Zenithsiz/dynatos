//! Signal
//!
//! A read-write value that automatically updates
//! any subscribers when changed.

// Imports
use {
	crate::{Effect, WeakEffect},
	std::{cell::RefCell, collections::HashSet, mem, rc::Rc},
};

/// Signal inner
struct Inner<T> {
	/// Value
	value: T,

	/// Subscribers
	subscribers: HashSet<WeakEffect>,
}

/// Signal
pub struct Signal<T> {
	/// Inner
	inner: Rc<RefCell<Inner<T>>>,
}

impl<T> Signal<T> {
	/// Creates a new signal
	pub fn new(value: T) -> Self {
		let inner = Inner {
			value,
			subscribers: HashSet::new(),
		};
		Self {
			inner: Rc::new(RefCell::new(inner)),
		}
	}

	/// Gets the inner value
	#[track_caller]
	pub fn get(&self) -> T
	where
		T: Copy,
	{
		if let Some(effect) = Effect::running() {
			self.add_subscriber(effect);
		}

		self.inner
			.try_borrow()
			.expect("Cannot get signal value while updating")
			.value
	}

	/// Calls `f` with the inner value
	#[track_caller]
	pub fn with<O, F>(&self, f: F) -> O
	where
		F: FnOnce(&T) -> O,
	{
		let inner = self.inner.try_borrow().expect("Cannot use signal value while updating");
		f(&inner.value)
	}

	/// Sets the inner value.
	///
	/// Updates all subscribers.
	///
	/// Returns the previous value
	pub fn set(&self, new_value: T) -> T {
		self.update(|value| mem::replace(value, new_value))
	}

	/// Updates the value in-place.
	///
	/// Updates all subscribers
	#[track_caller]
	pub fn update<O, F>(&self, f: F) -> O
	where
		F: FnOnce(&mut T) -> O,
	{
		// Update the value and get the output
		let output = {
			let mut inner = self
				.inner
				.try_borrow_mut()
				.expect("Cannot update signal value while using it");
			f(&mut inner.value)
		};

		// Then update all subscribers, removing any stale ones.
		// Note: Since running the effect will add subscribers, we can't keep
		//       the inner borrow active, so we gather all dependencies before-hand.
		//       However, we can remove subscribers in between running effects, so we
		//       don't need to wait for that.
		let subscribers = self.inner.borrow().subscribers.iter().cloned().collect::<Vec<_>>();
		for subscriber in subscribers {
			if !subscriber.try_run() {
				self.remove_subscriber(subscriber);
			}
		}

		output
	}

	/// Explicitly adds a subscriber to this signal.
	///
	/// Returns if the subscriber already existed.
	pub fn add_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		let mut inner = self.inner.borrow_mut();
		let new_effect = inner.subscribers.insert(subscriber.into_subscriber());
		!new_effect
	}

	/// Removes a subscriber from this signal.
	///
	/// Returns if the subscriber existed
	pub fn remove_subscriber<S: IntoSubscriber>(&self, subscriber: S) -> bool {
		let mut inner = self.inner.borrow_mut();
		inner.subscribers.remove(&subscriber.into_subscriber())
	}
}

impl<T> Clone for Signal<T> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

/// Types that may be converted into a subscriber
pub trait IntoSubscriber {
	fn into_subscriber(self) -> WeakEffect;
}

#[duplicate::duplicate_item(
	T body;
	[ Effect ] [ self.downgrade() ];
	[ &'_ Effect ] [ self.downgrade() ];
	[ WeakEffect ] [ self ];
)]
impl IntoSubscriber for T {
	fn into_subscriber(self) -> WeakEffect {
		body
	}
}
