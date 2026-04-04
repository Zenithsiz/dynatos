//! Value downcasting

// Imports
use crate::{Base, Value};

/// Value downcasting
pub const trait Downcast: Value {
	/// Attempts to downcast this value into `T`.
	fn downcast<T: [const] Value>(self) -> Result<T, Self>;

	/// Attempts to downcast a reference to this value into `&T`.
	fn downcast_ref<T: [const] Value>(&self) -> Option<&T>;
}

impl<T: [const] Value> const Downcast for T {
	fn downcast<U: [const] Value>(self) -> Result<U, Self> {
		let base = Base::from_value(self);
		match base.is::<U>() || base.has_parent::<U>() {
			// SAFETY: We just checked that `base` is a valid value for `U`
			true => Ok(unsafe { U::from_repr(base) }),

			// SAFETY: `from_repr(into_repr)` is a no-op.
			false => Err(unsafe { Self::from_repr(base) }),
		}
	}

	fn downcast_ref<U: [const] Value>(&self) -> Option<&U> {
		let base = Base::from_value_ref(self);
		match base.is::<U>() || base.has_parent::<U>() {
			// SAFETY: We just checked that `base` is a valid value for `U`
			true => Some(unsafe { U::from_ref(base) }),
			false => None,
		}
	}
}
