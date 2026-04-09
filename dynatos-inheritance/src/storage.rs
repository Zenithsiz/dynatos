//! Value storage

// Imports
use {
	crate::{DebugFields, StorageFromFields, ref_count::RefCount},
	core::fmt,
};

/// Base storage for values
#[derive(Debug)]
pub struct BaseStorage {
	pub(crate) ref_count: RefCount,
}

impl BaseStorage {
	/// Creates new value storage
	#[must_use]
	pub const fn new() -> Self {
		Self {
			ref_count: RefCount::new(),
		}
	}
}

/// This impl just calls [`Self::new`]. It is intended
/// to be used when cloning a whole storage to create
/// a new object
impl const Clone for BaseStorage {
	fn clone(&self) -> Self {
		Self::new()
	}
}

impl Default for BaseStorage {
	fn default() -> Self {
		Self::new()
	}
}

impl const AsRef<()> for BaseStorage {
	fn as_ref(&self) -> &() {
		&()
	}
}

impl const StorageFromFields for BaseStorage {
	type Fields = ();

	fn from_fields(base: BaseStorage, _fields: Self::Fields) -> Self {
		base
	}
}

impl DebugFields for BaseStorage {
	fn debug_fields(&self, _s: &mut fmt::DebugStruct<'_, '_>) {}
}
