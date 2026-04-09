//! Base vtable

// Imports
use {
	crate::{BaseStorage, Contains, DebugFields, VTableFromMethods, Value},
	core::{
		alloc::Layout,
		any::{self, TypeId},
		fmt,
		ptr::NonNull,
	},
};

/// Base vtable for values
#[derive(Clone, Copy, Debug)]
pub struct BaseVTable {
	pub(crate) drop_storage:   unsafe fn(NonNull<BaseStorage>),
	pub(crate) storage_layout: Layout,
	pub(crate) debug:          unsafe fn(NonNull<BaseStorage>, &mut fmt::DebugStruct<'_, '_>),
	pub(crate) ty:             TypeId,
	pub(crate) parents:        &'static [TypeId],
}

impl BaseVTable {
	/// Creates a new vtable for `T`
	#[must_use]
	pub const fn new<T: Value>() -> Self {
		Self {
			drop_storage:   Self::drop_storage::<T>,
			storage_layout: Layout::new::<T::Storage>(),
			debug:          Self::debug::<T>,
			ty:             TypeId::of::<T>(),
			parents:        T::PARENTS,
		}
	}

	unsafe fn drop_storage<T: Value>(storage: NonNull<BaseStorage>) {
		let storage_ptr = <T::Storage as Contains<BaseStorage>>::from_non_null(storage);

		// SAFETY: We allocated a `T::Storage` in `self` that we're retrieving now.
		//         There aren't any other references to this value currently.
		drop(unsafe { storage_ptr.read() });
	}

	unsafe fn debug<T: Value>(storage: NonNull<BaseStorage>, s: &mut fmt::DebugStruct<'_, '_>) {
		let storage_ptr = <T::Storage as Contains<BaseStorage>>::from_non_null(storage);

		// SAFETY: We allocated a `T::Storage` in `self` that we're retrieving now.
		let storage = unsafe { storage_ptr.as_ref() };

		if let Some(storage_debug) = any::try_as_dyn::<_, dyn DebugFields>(storage) {
			storage_debug.debug_fields(s);
		}
	}
}

impl const AsRef<()> for BaseVTable {
	fn as_ref(&self) -> &() {
		&()
	}
}

impl const VTableFromMethods for BaseVTable {
	type Methods = ();

	fn from_methods(base: BaseVTable, _methods: Self::Methods) -> Self {
		base
	}
}
