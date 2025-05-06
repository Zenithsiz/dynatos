//! [`SignalGet`]

// Imports
use {
	crate::SignalWith,
	core::{any::TypeId, mem},
};

/// Types which may be copied by [`SignalGet`]
pub trait SignalGetCopy: Sized {
	type Value: 'static;
	fn copy_value(self) -> Self::Value;
}

impl<T: Copy + 'static> SignalGetCopy for &'_ T {
	type Value = T;

	fn copy_value(self) -> Self::Value {
		*self
	}
}
impl<T: Copy + 'static> SignalGetCopy for Option<&'_ T> {
	type Value = Option<T>;

	fn copy_value(self) -> Self::Value {
		self.copied()
	}
}

/// Signal get
pub trait SignalGet {
	/// Value type
	type Value;

	/// Gets the signal value, by copying it.
	fn get(&self) -> Self::Value;
}

impl<S> SignalGet for S
where
	S: for<'a> SignalWith<Value<'a>: SignalGetCopy> + 'static,
{
	type Value = <S::Value<'static> as SignalGetCopy>::Value;

	#[track_caller]
	fn get(&self) -> Self::Value {
		self.with(|value| self::convert_inner::<S>(value.copy_value()))
	}
}

/// Converts the value of a specific lifetime `SignalGetCopy` to the `'static` one.
#[duplicate::duplicate_item(
	From To;
	[<S::Value<'a> as SignalGetCopy>::Value]
	[<S::Value<'static> as SignalGetCopy>::Value];
)]
fn convert_inner<'a, S>(value: From) -> To
where
	S: for<'b> SignalWith<Value<'b>: SignalGetCopy> + 'static,
{
	debug_assert_eq!(
		TypeId::of::<From>(),
		TypeId::of::<To>(),
		"GAT with `'static` lifetime was different than `'a`"
	);

	// SAFETY: You cannot specialize on lifetimes, and
	//         `SignalGetCopy::Value: 'static`, so both
	//         types must be the same type. We also verify
	//         that the types are the same before-hand
	unsafe { mem::transmute::<From, To>(value) }
}
