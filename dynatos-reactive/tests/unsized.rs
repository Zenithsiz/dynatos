//! `!Sized` tests.

// Imports
use {
	core::any::Any,
	dynatos_reactive::{Signal, SignalWith},
};

#[test]
fn create_unsized() {
	let value: i32 = 5;
	let sig = Signal::<i32>::new(value) as Signal<dyn Any>;

	sig.with(|dyn_value| {
		assert_eq!(dyn_value.type_id(), value.type_id());
		assert_eq!(dyn_value.downcast_ref::<i32>(), Some(&value));
	});
}
