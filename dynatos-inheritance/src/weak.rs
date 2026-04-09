//! Weak values

// Imports
use {
	crate::{Base, BaseStorage, BaseVTable, Value},
	core::{alloc::Allocator, fmt, marker::PhantomData, ptr::NonNull},
	std::alloc::Global,
};

/// Weak value reference
pub struct WeakValue<T> {
	storage: NonNull<BaseStorage>,
	vtable:  NonNull<BaseVTable>,
	phantom: PhantomData<T>,
}

impl<T: Value> WeakValue<T> {
	/// Creates a weak reference to a value
	pub fn new(value: &T) -> Self {
		let base = value.as_ref();

		base.storage().ref_count.inc_weak();
		Self {
			storage: base.storage,
			vtable:  base.vtable,
			phantom: PhantomData,
		}
	}

	/// Upgrades this reference to a strong reference
	#[must_use]
	pub fn upgrade(&self) -> Option<T> {
		match self.storage().ref_count.inc_strong_non0() {
			true => {
				let base = Base {
					storage: self.storage,
					vtable:  self.vtable,
				};

				// SAFETY: We were created with a `T`, and we just incremented
				//         the strong count, so this is safe.
				let value = unsafe { T::from_repr(base) };
				Some(value)
			},
			false => None,
		}
	}
}

impl<T> WeakValue<T> {
	/// Returns the base storage behind this weak reference
	pub(crate) const fn storage(&self) -> &BaseStorage {
		// SAFETY: We ensure our storage always exists
		unsafe { self.storage.as_ref() }
	}

	/// Returns the vtable behind this weak reference
	pub(crate) const fn vtable(&self) -> &BaseVTable {
		// SAFETY: We ensure our vtable always exists
		unsafe { self.vtable.as_ref() }
	}

	/// Prints debug information about the value.
	pub(crate) fn fmt_debug(&self, s: &mut fmt::DebugStruct<'_, '_>) {
		// SAFETY: The storage is valid.
		unsafe { (self.vtable().debug)(self.storage, s) };
	}
}

impl<T> Clone for WeakValue<T> {
	fn clone(&self) -> Self {
		self.storage().ref_count.inc_weak();
		Self {
			storage: self.storage,
			vtable:  self.vtable,
			phantom: PhantomData,
		}
	}
}

impl<T> PartialEq for WeakValue<T> {
	fn eq(&self, other: &Self) -> bool {
		self.storage == other.storage
	}
}

impl<T: Value> PartialEq<T> for WeakValue<T> {
	fn eq(&self, other: &T) -> bool {
		self.storage == other.as_ref().storage
	}
}

impl<T> fmt::Debug for WeakValue<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("Base");
		self.fmt_debug(&mut s);
		s.finish()
	}
}

impl<T> Drop for WeakValue<T> {
	fn drop(&mut self) {
		// If were the last weak reference, we can deallocate
		if self.storage().ref_count.dec_weak() {
			// SAFETY: We ensure our vtable always exists
			let vtable = unsafe { self.vtable.as_ref() };

			// SAFETY: No more reference exist to the value, so we can
			//         deallocate the allocation.
			unsafe { Global.deallocate(self.storage.cast(), vtable.storage_layout) };
		}
	}
}

// SAFETY: We're Send/Sync if our base value is too
unsafe impl<T: Send> Send for WeakValue<T> {}
// SAFETY: See above
unsafe impl<T: Sync> Sync for WeakValue<T> {}

pub trait ValueDowngrade: Value {
	/// Downgrades this value into a weak reference
	#[must_use]
	fn downgrade(&self) -> WeakValue<Self> {
		WeakValue::new(self)
	}
}

impl<T: Value> ValueDowngrade for T {}
