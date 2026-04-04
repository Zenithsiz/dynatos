//! Object extension

// Imports
use crate::{Base, ReprTransparent, Value};

/// Extends this object to include a parent
pub trait Extend: Value {
	fn extend_with_fields<T: Value>(self, fields: T::Fields) -> Result<T, Self>
	where
		Self::Storage: ExtendStorage<T>;
}

impl<T: Value> Extend for T {
	fn extend_with_fields<U: Value>(self, fields: U::Fields) -> Result<U, Self>
	where
		Self::Storage: ExtendStorage<U>,
	{
		let base = Base::from_value(self);
		let storage = match base.into_storage_of::<Self>() {
			Ok(storage) => storage,
			// SAFETY: The value is valid since we just got it from `self`
			Err(base) => return Err(unsafe { ReprTransparent::from_repr(base) }),
		};

		let storage = <Self::Storage as ExtendStorage<U>>::extend_with_fields(storage, fields);

		Ok(U::from_storage(storage))
	}
}

/// Extends this storage to include more fields
pub const trait ExtendStorage<T: Value>: Sized {
	fn extend_with_fields(self, fields: T::Fields) -> T::Storage;
}
