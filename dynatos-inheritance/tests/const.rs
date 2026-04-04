//! Const tests

// Features
#![feature(
	decl_macro,
	const_trait_impl,
	const_cmp,
	const_index,
	const_clone,
	more_qualified_paths,
	macro_metavar_expr,
	macro_metavar_expr_concat,
	trivial_bounds,
	const_convert
)]

// Imports
use dynatos_inheritance::{CloneStorage, Downcast, FromFields};

dynatos_inheritance::value! {
	struct A(): Const + CloneStorage + Send + Sync {}
	impl Self {}
}

dynatos_inheritance::value! {
	struct B(A): Const + CloneStorage + Send + Sync {}
	impl Self {}
}

const fn _from_fields() -> B {
	B::from_fields((BFields {}, AFields {}))
}

const fn _downcast(b: B) -> Result<A, B> {
	b.downcast()
}

const fn _clone_storage(b: &B) -> B {
	b.clone_storage()
}

const fn _into_parent(b: B) -> A {
	A::from(b)
}

const fn _as_parent(b: &B) -> &A {
	b.as_ref()
}
