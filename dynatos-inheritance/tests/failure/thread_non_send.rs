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
use {
	dynatos_inheritance::Value,
	std::{cell::RefCell, thread},
};

dynatos_inheritance::value! {
	struct A() {
		a: RefCell<u32>,
	}
	impl Self {}
}

fn send_a(a: A) {
	thread::spawn(move || {
		let _ = a.fields().a.borrow_mut();
	});
}

fn main() {}
