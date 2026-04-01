//! Atomic counter

// Imports
use core::sync::atomic::{self, AtomicUsize};

/// Atomic counter
pub struct Counter(AtomicUsize);

impl Counter {
	#[must_use]
	pub const fn new() -> Self {
		Self(AtomicUsize::new(0))
	}

	pub fn bump(&self) {
		self.0.fetch_add(1, atomic::Ordering::AcqRel);
	}

	pub fn get(&self) -> usize {
		self.0.load(atomic::Ordering::Acquire)
	}
}

impl Default for Counter {
	fn default() -> Self {
		Self::new()
	}
}
