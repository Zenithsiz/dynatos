//! Signal
//!
//! A read-write value that automatically updates
//! any subscribers when changed.

// Imports
use {
	crate::{Effect, SignalGet, SignalReplace, SignalSet, SignalUpdate, SignalWith, Trigger, WeakEffect},
	std::{cell::RefCell, fmt, mem, rc::Rc},
};

/// Inner
struct Inner<T> {
	/// Value
	value: RefCell<T>,

	/// Trigger
	trigger: Trigger,
}

/// Signal
pub struct Signal<T> {
	/// Inner
	inner: Rc<Inner<T>>,
}

impl<T> Signal<T> {
	/// Creates a new signal
	pub fn new(value: T) -> Self {
		let inner = Inner {
			value:   RefCell::new(value),
			trigger: Trigger::new(),
		};
		Self { inner: Rc::new(inner) }
	}
}

impl<T> SignalGet for Signal<T>
where
	T: Copy,
{
	type Value = T;

	fn get(&self) -> Self::Value {
		self.with(|value| *value)
	}
}

impl<T> SignalWith for Signal<T> {
	type Value = T;

	fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Self::Value) -> O,
	{
		if let Some(effect) = Effect::running() {
			self.inner.trigger.add_subscriber(effect);
		}

		let value = self
			.inner
			.value
			.try_borrow()
			.expect("Cannot use signal value while updating");
		f(&value)
	}
}

impl<T> SignalSet<T> for Signal<T> {
	fn set(&self, new_value: T) {
		self.update(|value| *value = new_value);
	}
}

impl<T> SignalReplace<T> for Signal<T> {
	fn replace(&self, new_value: T) -> T {
		self.update(|value| mem::replace(value, new_value))
	}
}

impl<T> SignalUpdate for Signal<T> {
	type Value = T;

	fn update<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&mut Self::Value) -> O,
	{
		// Update the value and get the output
		let output = {
			let mut value = self
				.inner
				.value
				.try_borrow_mut()
				.expect("Cannot update signal value while using it");
			f(&mut value)
		};

		// Then trigger our trigger
		self.inner.trigger.trigger();

		output
	}
}

impl<T> Clone for Signal<T> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

impl<T: fmt::Debug> fmt::Debug for Signal<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Signal")
			.field("value", &*self.inner.value.borrow())
			.field("trigger", &self.inner.trigger)
			.finish()
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
