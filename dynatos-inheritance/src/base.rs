//! Base value

// Imports
use {
	crate::{BaseStorage, BaseVTable, CloneStorage, Contains, FromFields, ReprTransparent, Value, ValueFor},
	core::{
		alloc::{Allocator, Layout},
		any::TypeId,
		fmt,
		mem,
		ptr::NonNull,
		sync::atomic,
	},
	std::alloc::Global,
};

/// Base value for all inheritance values
pub struct Base {
	storage: NonNull<BaseStorage>,
	vtable:  NonNull<BaseVTable>,
}

impl Base {
	/// Creates a value from it's storage.
	pub const fn new<T>(storage: T::Storage) -> Self
	where
		T: [const] Value,
	{
		let storage_ptr = match Global.allocate(Layout::new::<T::Storage>()) {
			Ok(ptr) => ptr.cast::<T::Storage>(),
			Err(_) => panic!("Unable to allocate storage"),
		};
		// SAFETY: We just allocated the pointer
		unsafe { storage_ptr.write(storage) };

		let storage = Contains::to_non_null(storage_ptr);
		let vtable = Contains::to_non_null(NonNull::from_ref(T::VTABLE));

		Self { storage, vtable }
	}

	/// Gets a `&Base` from a `&impl Value`
	pub const fn from_value_ref<T: [const] Value>(value: &T) -> &Self {
		<T as ReprTransparent<Self>>::to_ref(value)
	}

	/// Gets a `Base` from a `impl Value`
	pub const fn from_value<T: [const] Value>(value: T) -> Self {
		<T as ReprTransparent<Self>>::into_repr(value)
	}

	/// Returns if this type is exactly a `T`.
	#[must_use]
	pub const fn is<T: 'static>(&self) -> bool {
		let ty = TypeId::of::<T>();

		let vtable = self.vtable();
		ty == vtable.ty
	}

	/// Returns if this type's parents contain a `T`
	#[must_use]
	pub const fn has_parent<T: 'static>(&self) -> bool {
		let ty = TypeId::of::<T>();

		let vtable = self.vtable();

		// TODO: once `[T]::contains` is `const` use that.
		let mut remaining_parents = vtable.parents;
		while let Some((&cur_parent, next_parents)) = remaining_parents.split_first() {
			if cur_parent == ty {
				return true;
			}
			remaining_parents = next_parents;
		}

		false
	}

	/// Gets a reference to the storage of `T` from this value.
	///
	/// # Safety
	/// `T` must be either the type this value was created with
	/// or one of it's parent types.
	#[must_use]
	pub const unsafe fn storage_of<T: [const] Value>(&self) -> &T::Storage {
		let storage = <T::Storage as Contains<BaseStorage>>::from_non_null(self.storage);

		// SAFETY: Caller ensures that a `T::Storage` exists.
		unsafe { storage.as_ref() }
	}

	/// Gets a reference to the fields of `T` from this value.
	///
	/// # Safety
	/// `T` must be either the type this value was created with
	/// or one of it's parent types.
	#[must_use]
	pub const unsafe fn fields_of<T: [const] Value>(&self) -> &T::Fields {
		// SAFETY: Caller ensures that a `T::Storage` exists.
		let storage = unsafe { self.storage_of::<T>() };
		storage.as_ref()
	}

	/// Gets a reference to the vtable of `T` from this value.
	///
	/// # Safety
	/// `T` must be either the type this value was created with
	/// or one of it's parent types.
	#[must_use]
	pub const unsafe fn vtable_of<T: [const] Value>(&self) -> &'static T::VTable {
		let vtable = <T::VTable as Contains<BaseVTable>>::from_non_null(self.vtable);

		// SAFETY: Caller ensures that a `T::VTable` exists.
		unsafe { vtable.as_ref() }
	}

	/// Consumes this value, returning it's storage.
	///
	/// `T` must be the type this value was created with.
	/// It *cannot* be one of it's parents, else this will
	/// return `Err`.
	pub fn into_storage_of<T: Value>(self) -> Result<T::Storage, Self> {
		// Note: If only 1 reference exists, then no more can be created,
		//       since we hold it.
		if !self.is::<T>() || self.storage().ref_count.load(atomic::Ordering::Acquire) != 1 {
			return Err(self);
		}

		let storage_ptr = <T::Storage as Contains<BaseStorage>>::from_non_null(self.storage);
		// SAFETY: We allocated a `T::Storage` in `self` that we're retrieving now.
		//         There aren't any other references to this value currently.
		let storage = unsafe { storage_ptr.read() };

		// SAFETY: See above. We also ensure that we don't double-free it by forgetting `self`.
		unsafe { Global.deallocate(storage_ptr.cast(), Layout::new::<T::Storage>()) };
		mem::forget(self);

		Ok(storage)
	}

	/// Prints debug information about the value.
	pub fn fmt_debug(&self, s: &mut fmt::DebugStruct<'_, '_>) {
		// SAFETY: The storage is valid.
		unsafe { (self.vtable().debug)(self.storage, s) };
	}
}

impl const Value for Base {
	type Fields = ();
	type Methods = ();
	type Parent = Self;
	type Storage = BaseStorage;
	type VTable = BaseVTable;

	const PARENTS: &'static [TypeId] = &[];
	const VTABLE: &'static Self::VTable = &BaseVTable::new::<Self>();
}

impl<T> ValueFor<T> for Base {}

impl const AsRef<Self> for Base {
	fn as_ref(&self) -> &Self {
		self
	}
}

impl FromFields for Base {
	type Fields = ();

	fn from_fields((): Self::Fields) -> Self {
		Self::from_storage(BaseStorage::new())
	}
}

impl CloneStorage for Base {
	fn clone_storage(&self) -> Self {
		Self::from_fields(())
	}
}

impl Clone for Base {
	fn clone(&self) -> Self {
		self.storage().ref_count.fetch_add(1, atomic::Ordering::AcqRel);
		Self {
			storage: self.storage,
			vtable:  self.vtable,
		}
	}
}

impl PartialEq for Base {
	fn eq(&self, other: &Self) -> bool {
		self.storage == other.storage
	}
}

impl Eq for Base {}

impl fmt::Debug for Base {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("Base");
		self.fmt_debug(&mut s);
		s.finish()
	}
}

impl Drop for Base {
	fn drop(&mut self) {
		let ref_count = self.storage().ref_count.fetch_sub(1, atomic::Ordering::AcqRel);
		if ref_count == 1 {
			let vtable = self.vtable();

			// SAFETY: No more reference exist to the value, so we can
			//         drop it safely.
			unsafe { (vtable.drop)(self.storage) };
		}
	}
}
