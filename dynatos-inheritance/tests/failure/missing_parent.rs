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

dynatos_inheritance::value! {
	struct A() {}
	impl Self {}
}

dynatos_inheritance::value! {
	struct B(A) {}
	impl Self {}
}

dynatos_inheritance::value! {
	struct C(B) {}
	impl Self {}
}

fn main() {}
