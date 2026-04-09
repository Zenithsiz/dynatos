#![feature(
	macro_metavar_expr,
	macro_metavar_expr_concat,
	const_trait_impl,
	const_index,
	const_cmp,
	more_qualified_paths,
	trivial_bounds,
	unsize
)]

// Imports
use std::cell::RefCell;

dynatos_inheritance::value! {
	struct A(): Send + Sync {
	}
	impl Self {}
}

dynatos_inheritance::value! {
	struct B(A): Send + Sync {
		a: RefCell<u32>,
	}
	impl Self {}
}

fn main() {}
