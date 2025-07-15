//! Location

// Imports
use core::fmt;
#[cfg(debug_assertions)]
use core::panic::Location;

/// Location
#[derive(PartialEq, Eq, Clone, Copy, Hash)]
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

impl fmt::Debug for Loc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		#[cfg(debug_assertions)]
		{
			fmt::Display::fmt(&self.location, f)
		}

		#[cfg(not(debug_assertions))]
		f.pad("<optimized out>")
	}
}
