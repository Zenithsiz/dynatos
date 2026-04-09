//! Value

// Imports
use {
	crate::{Base, BaseStorage, BaseVTable, Contains, ReprTransparent},
	core::any::TypeId,
};

/// A value that is part of an inheritance chain.
// TODO: Does this need to be an `unsafe trait`?
//       We encode all the layout with `ReprTransparent` and
//       `Contains`, but there could be some hidden unsoundness.
pub const trait Value: [const] ReprTransparent<Base> + [const] AsRef<Base> + Sized + 'static {
	/// Fields
	type Fields;

	/// Methods
	type Methods;

	/// Storage.
	type Storage: [const] Contains<<Self::Parent as Value>::Storage>
		+ [const] Contains<BaseStorage>
		+ [const] AsRef<Self::Fields>;

	/// `VTable`
	type VTable: [const] Contains<<Self::Parent as Value>::VTable>
		+ [const] Contains<BaseVTable>
		+ [const] AsRef<Self::Methods>;

	/// Parent value
	type Parent: ValueFor<Self>;

	/// Parents of this type.
	const PARENTS: &'static [TypeId];

	/// `VTable` of this type
	const VTABLE: &'static Self::VTable;

	/// Returns a reference to the storage
	fn storage(&self) -> &Self::Storage {
		let base = Base::from_value_ref(self);

		// SAFETY: `Self` always has a valid `Self::Storage`.
		unsafe { base.storage_of::<Self>() }
	}

	/// Returns a reference to the fields
	fn fields(&self) -> &Self::Fields {
		let base = Base::from_value_ref(self);

		// SAFETY: `Self` always has a valid `Self::Storage`.
		unsafe { base.fields_of::<Self>() }
	}

	/// Returns a reference to the vtable
	fn vtable(&self) -> &'static Self::VTable {
		let base = Base::from_value_ref(self);

		// SAFETY: `Self` always has a valid `Self::Storage`.
		unsafe { base.vtable_of::<Self>() }
	}

	/// Creates this value from it's storage
	fn from_storage(storage: Self::Storage) -> Self {
		let base = Base::new::<Self>(storage);

		// SAFETY: The value was created with `Self::Storage`,
		//         and is thus valid for `Self`.
		unsafe { Self::from_repr(base) }
	}
}

/// Trait implemented for types that allow `T` as a child value
pub trait ValueFor<T>: Value {}
