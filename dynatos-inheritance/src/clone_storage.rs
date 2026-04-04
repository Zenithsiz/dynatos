//! Storage cloning

/// Clones the storage of a value into a new value.
///
/// # Slicing
/// This clone will only clone the storage of `Self`,
/// not any storage of it's parents, and will thus
/// "slice" the object.
///
/// To avoid this, add a virtual function to your type
/// to ensure that all parents need to implement it by
/// cloning using their respective type.
pub const trait CloneStorage {
	#[must_use]
	fn clone_storage(&self) -> Self;
}
