//! Dynamic predicates

// Imports
use {
	core::ops::Deref,
	dynatos_reactive::{Derived, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun},
};

/// Values that may be used as possible dynamic predicates.
///
/// This allows it to work with the following types:
/// - `bool`
/// - `Signal<B>`
/// - `impl Fn() -> B`
///
/// Where `B` is any of the types above.
pub trait WithDynPred {
	/// Evaluates this predicate
	fn eval(&self) -> bool;
}

impl<FT, T> WithDynPred for FT
where
	FT: Fn() -> T,
	T: WithDynPred,
{
	fn eval(&self) -> bool {
		self().eval()
	}
}

impl WithDynPred for bool {
	fn eval(&self) -> bool {
		*self
	}
}

// TODO: Allow impl for `impl SignalGet<Value: WithDynText>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: WithDynPred + 'static];
	[T, F] [Derived<T, F> where T: WithDynPred + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: WithDynPred + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Deref<Target: WithDynPred>>];
)]
impl<Generics> WithDynPred for Ty {
	fn eval(&self) -> bool {
		#[allow(
			clippy::allow_attributes,
			clippy::redundant_closure_for_method_calls,
			reason = "In some branches it isn't redundant"
		)]
		self.with(|value| value.eval())
	}
}
