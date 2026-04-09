//! Weak tests

// Features
#![feature(const_trait_impl, more_qualified_paths, trivial_bounds)]

// Imports
use dynatos_inheritance::ValueDowngrade;

dynatos_inheritance::value! {
	struct A(): Default {}
	impl Self {}
}

dynatos_inheritance::value! {
	struct B(A): Default {}
	impl Self {}
}

#[test]
fn upgrade_weak() {
	let b = B::default();
	let weak_b = b.downgrade();

	assert_eq!(weak_b.upgrade(), Some(b));
}

#[test]
fn weak_upgrade_after_drop() {
	let b = B::default();
	let weak_b = b.downgrade();
	drop(b);

	assert_eq!(weak_b.upgrade(), None);
}
