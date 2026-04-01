//! Utilities for `dynatos`

// Features
#![feature(decl_macro, never_type, try_trait_v2, option_reference_flattening)]

// Modules
pub mod counter;
pub mod holey_stack;
pub mod try_or_return;

// Exports
pub use self::{
	counter::Counter,
	holey_stack::HoleyStack,
	try_or_return::{TryOrReturn, TryOrReturnExt},
};

// Imports
use {
	core::hash::{self, Hasher},
	std::hash::DefaultHasher,
};

/// Calculates the hash of a value using the default hasher
pub fn hash_of<T: hash::Hash>(t: &T) -> u64 {
	let mut s = DefaultHasher::new();
	t.hash(&mut s);
	s.finish()
}
