//! Utilities for `dynatos`

// Features
#![feature(try_trait_v2, try_trait_v2_residual, option_reference_flattening, decl_macro)]

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

pub mod web {
	/// Dynatos web feature selector for expressions.
	pub macro cfg_ssr_expr(ssr = $ssr:expr,csr = $csr:expr $(,)?) {
		cfg_ssr! {
			ssr = { $ssr },
			csr = { $csr },
		}
	}

	/// Dynatos web feature selector.
	// TODO: Use `=>` instead of `=` when formatting still happens with it.
	pub macro cfg_ssr(ssr = $ssr:tt,csr = $csr:tt $(,)?) {
		cfg_select! {
			all(feature = "ssr", feature = "csr") => {
				compile_error! { "The `ssr` and `csr` features are mutually exclusive" }
			}

			feature = "ssr" => $ssr,
			feature = "csr" => $csr,

			_ => {
				compile_error! { "At least one of the `ssr` or `csr` features must be enabled" }
			}
		}
	}
}