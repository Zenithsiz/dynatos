//! Extension

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
use dynatos_inheritance::{Extend, FromFields, Value};

dynatos_inheritance::value! {
	struct A() {
		string: String,
	}
	impl Self {}
}

impl A {
	pub fn default_fields() -> <Self as Value>::Fields {
		<Self as Value>::Fields { string: "A".to_owned() }
	}
}

impl Default for A {
	fn default() -> Self {
		Self::from_fields((Self::default_fields(),))
	}
}

dynatos_inheritance::value! {
	struct B(A) {
		list: Vec<&'static str>,
	}
	impl Self {}
}

impl B {
	pub fn default_fields() -> <Self as Value>::Fields {
		<Self as Value>::Fields {
			list: vec!["A", "B", "C"],
		}
	}
}

impl Default for B {
	fn default() -> Self {
		Self::from_fields((Self::default_fields(), A::default_fields()))
	}
}

dynatos_inheritance::value! {
	struct C(B, A) {}
	impl Self {}
}

#[test]
fn extend_cloned() {
	let a = A::default();
	let _a2 = a.clone();
	a.extend_with_fields(B::default_fields())
		.expect_err("Should not be able to extend while clones exist");
}

#[test]
fn extend() {
	let a = A::default();
	let _b = a.extend_with_fields(B::default_fields()).expect("Unable to extend");
}

#[test]
fn extend_parent() {
	let b = B::default();
	let a = A::from(b);

	a.extend_with_fields(B::default_fields())
		.expect_err("Should not be able to extend from parent");
}
