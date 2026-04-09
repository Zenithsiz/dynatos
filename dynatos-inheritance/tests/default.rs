//! Default tests

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

dynatos_inheritance::value! {
	struct A(): Default {}
	impl Self {}
}

dynatos_inheritance::value! {
	struct B(A): Default {}
	impl Self {}
}

dynatos_inheritance::value! {
	struct C(A): DefaultFields {}
	impl Self {}
}

#[test]
fn default() {
	let _ = AFields::default();
	let _ = AStorage::default();
	let _ = A::default();
	let _ = BFields::default();
	let _ = BStorage::default();
	let _ = B::default();
	let _ = CFields::default();
}
