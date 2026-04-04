//! Debug utilities

// Imports
use core::fmt;

/// Debugs all fields into `s`
pub trait DebugFields {
	fn debug_fields(&self, s: &mut fmt::DebugStruct<'_, '_>);
}
