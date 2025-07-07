//! Location

// Imports
#[cfg(debug_assertions)]
use core::panic::Location;

/// Location
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
#[derive(derive_more::Display)]
pub struct Loc {
	/// Inner location
	#[cfg(debug_assertions)]
	location: &'static Location<'static>,
}

impl Loc {
	/// Gets the caller's location
	#[track_caller]
	pub const fn caller() -> Self {
		Self {
			#[cfg(debug_assertions)]
			location:                          Location::caller(),
		}
	}
}
