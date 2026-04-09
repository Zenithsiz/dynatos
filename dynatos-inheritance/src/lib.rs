//! Inheritance for `dynatos`

// Features
#![feature(
	const_trait_impl,
	const_convert,
	const_heap,
	const_cmp,
	const_clone,
	allocator_api,
	try_as_dyn
)]

// Modules
mod base;
mod clone_storage;
mod debug;
mod downcast;
mod extend;
mod ref_count;
mod storage;
mod util;
mod value;
mod vtable;

// Exports
pub use {
	self::{
		base::Base,
		clone_storage::CloneStorage,
		debug::DebugFields,
		downcast::Downcast,
		extend::{Extend, ExtendStorage},
		storage::BaseStorage,
		util::{Contains, ReprTransparent},
		value::{Value, ValueFor},
		vtable::BaseVTable,
	},
	dynatos_inheritance_macros::value,
};

/// Creates a value from it's fields.
pub const trait FromFields {
	type Fields;

	fn from_fields(fields: Self::Fields) -> Self;
}

/// Creates a storage from it's fields
pub const trait StorageFromFields {
	type Fields;

	fn from_fields(base: BaseStorage, fields: Self::Fields) -> Self;
}

/// Creates a vtable from it's methods
pub const trait VTableFromMethods {
	type Methods;

	fn from_methods(base: BaseVTable, methods: Self::Methods) -> Self;
}
