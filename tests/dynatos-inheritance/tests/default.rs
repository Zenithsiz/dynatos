//! Default tests

// Features
#![feature(const_trait_impl, more_qualified_paths, trivial_bounds)]

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
