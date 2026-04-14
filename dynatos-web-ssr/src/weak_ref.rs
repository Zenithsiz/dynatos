//! Weak reference

// Imports
use dynatos_inheritance::{Value, ValueDowngrade, WeakValue};

#[derive(Debug)]
pub struct WeakRef<T>(WeakValue<T>);

impl<T: Value> WeakRef<T> {
	pub fn new(value: &T) -> Self {
		Self(value.downgrade())
	}

	#[must_use]
	pub fn deref(&self) -> Option<T> {
		self.0.upgrade()
	}
}
