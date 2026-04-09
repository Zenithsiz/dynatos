//! Atomic reference count

// Imports
use core::sync::atomic::{self, AtomicUsize};

/// Atomic reference count
#[derive(Debug)]
pub struct RefCount {
	strong: AtomicUsize,
}

impl RefCount {
	pub const fn new() -> Self {
		Self {
			strong: AtomicUsize::new(1),
		}
	}

	/// Returns if this reference count is unique
	pub fn is_unique(&self) -> bool {
		self.strong.load(atomic::Ordering::Acquire) == 1
	}

	/// Adds a strong reference
	pub fn inc_strong(&self) {
		self.strong.fetch_add(1, atomic::Ordering::AcqRel);
	}

	/// Decrements a strong reference.
	///
	/// Returns if this is the last reference
	pub fn dec_strong(&self) -> bool {
		self.strong.fetch_sub(1, atomic::Ordering::AcqRel) == 1
	}
}
