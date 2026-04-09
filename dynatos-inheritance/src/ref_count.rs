//! Atomic reference count

// Imports
use core::sync::atomic::{self, AtomicUsize};

/// Atomic reference count
#[derive(Debug)]
pub struct RefCount {
	strong: AtomicUsize,
	weak:   AtomicUsize,
}

impl RefCount {
	/// Creates a new reference count.
	///
	/// This will contain both 1 strong and 1 weak reference.
	pub const fn new() -> Self {
		Self {
			strong: AtomicUsize::new(1),
			weak:   AtomicUsize::new(1),
		}
	}

	/// Returns if this reference count is unique
	pub fn is_unique(&self) -> bool {
		// Note: Order is important. If we checked weak first, it is possible for
		//       another strong to create a weak and be dropped in the meantime, but
		//       by checking strong first, we know there aren't any others to create
		//       weak references.
		self.strong.load(atomic::Ordering::Acquire) == 1 && self.weak.load(atomic::Ordering::Acquire) == 1
	}

	/// Adds a strong reference
	pub fn inc_strong(&self) {
		self.strong.fetch_add(1, atomic::Ordering::AcqRel);
	}

	/// Decrements a strong reference.
	///
	/// Returns if this is the last strong reference.
	pub fn dec_strong(&self) -> bool {
		self.strong.fetch_sub(1, atomic::Ordering::AcqRel) == 1
	}

	/// Increments a strong reference, if non-0.
	///
	/// Returns if successful
	pub fn inc_strong_non0(&self) -> bool {
		self.strong
			.try_update(atomic::Ordering::Acquire, atomic::Ordering::Relaxed, |strong| {
				match strong == 0 {
					true => None,
					false => Some(strong + 1),
				}
			})
			.is_ok()
	}

	/// Increments the weak reference count
	pub fn inc_weak(&self) {
		self.weak.fetch_add(1, atomic::Ordering::AcqRel);
	}

	/// Decreases the weak reference count
	///
	/// Returns if this is the last reference (including weak references)
	pub fn dec_weak(&self) -> bool {
		self.weak.fetch_sub(1, atomic::Ordering::AcqRel) == 1
	}
}
