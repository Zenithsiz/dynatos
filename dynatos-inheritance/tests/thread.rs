//! Thread tests

// Features
#![feature(
	box_vec_non_null,
	decl_macro,
	const_trait_impl,
	const_cmp,
	const_index,
	more_qualified_paths,
	macro_metavar_expr,
	macro_metavar_expr_concat,
	trivial_bounds,
	unsize
)]

// Imports
use {dynatos_inheritance::FromFields, std::thread};

dynatos_inheritance::value! {
	struct A(): Send + Sync {}
	impl Self {}
}

dynatos_inheritance::value! {
	struct B(A): Send + Sync {}
	impl Self {}
}

dynatos_inheritance::value! {
	struct C(B, A): Send + Sync {}
	impl Self {}
}

#[test]
fn drop_on_other_thread() {
	let a = A::from_fields((AFields {},));
	thread::spawn(|| {
		let _a: A = a;
	});
}
