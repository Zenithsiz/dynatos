//! `dynatos` web types

// Features
#![feature(trait_alias, decl_macro)]

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

cfg_ssr! {
	ssr = {
		mod ssr;
		pub use self::ssr::*;
	},
	csr = {
		mod csr;
		pub use self::csr::*;
	},
}
