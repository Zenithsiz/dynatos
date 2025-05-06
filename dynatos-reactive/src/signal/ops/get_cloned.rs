//! [`SignalGetCloned`]

// Imports
use {
	crate::SignalWith,
	core::{any::TypeId, mem},
};

/// Types which may be cloned by [`SignalGetCloned`]
pub trait SignalGetClone: Sized {
	type Value: 'static;
	fn clone_value(self) -> Self::Value;
}

impl<T: Clone + 'static> SignalGetClone for &'_ T {
	type Value = T;

	fn clone_value(self) -> Self::Value {
		self.clone()
	}
}
impl<T: Clone + 'static> SignalGetClone for Option<&'_ T> {
	type Value = Option<T>;

	fn clone_value(self) -> Self::Value {
		self.cloned()
	}
}

/// Signal cloned
pub trait SignalGetCloned {
	/// Value type
	type Value;

	/// Gets the signal value, by cloning it.
	fn get_cloned(&self) -> Self::Value;
}

impl<S> SignalGetCloned for S
where
	S: for<'a> SignalWith<Value<'a>: SignalGetClone> + 'static,
{
	type Value = <S::Value<'static> as SignalGetClone>::Value;

	#[track_caller]
	fn get_cloned(&self) -> Self::Value {
		self.with(|value| self::convert_inner::<S>(value.clone_value()))
	}
}

/// Converts the value of a specific lifetime `SignalGetClone` to the `'static` one.
#[duplicate::duplicate_item(
	From To;
	[<S::Value<'a> as SignalGetClone>::Value]
	[<S::Value<'static> as SignalGetClone>::Value];
)]
fn convert_inner<'a, S>(value: From) -> To
where
	S: for<'b> SignalWith<Value<'b>: SignalGetClone> + 'static,
{
	debug_assert_eq!(
		TypeId::of::<From>(),
		TypeId::of::<To>(),
		"GAT with `'static` lifetime was different than `'a`"
	);

	// SAFETY: You cannot specialize on lifetimes, and
	//         `SignalGetClone::Value: 'static`, so both
	//         types must be the same type. We also verify
	//         that the types are the same before-hand
	unsafe { mem::transmute::<From, To>(value) }
}
