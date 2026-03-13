//! Dynamic text

// Imports
use {
	core::ops::Deref,
	dynatos_reactive::{Derived, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun},
	dynatos_router::{QuerySignal, query_signal::QueryParse},
};

/// Values that may be used as possible dynamic text.
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `{str, &str, String}`
/// - `Option<N>`
/// - [`Signal`], [`Derived`], [`Memo`], [`WithDefault`]
/// - `LazyCell<N, impl Fn() -> N>`
///
/// Where `N` is any of the types above.
pub trait WithDynText {
	/// Calls `f` with the inner text
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O;
}

impl<FT, T> WithDynText for FT
where
	FT: Fn() -> T,
	T: WithDynText,
{
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		let text = self();
		text.with_text(f)
	}
}

#[allow(clippy::allow_attributes, reason = "This only applies in some branches")]
#[allow(clippy::use_self, reason = "We always want to use `&str`, not `&Ty`")]
#[duplicate::duplicate_item(
	Ty;
	[str];
	[&'_ str];
	[String];
)]
impl WithDynText for Ty {
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		f(Some(self))
	}
}

impl<T> WithDynText for Option<T>
where
	T: WithDynText,
{
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		match self {
			Some(s) => s.with_text(f),
			None => f(None),
		}
	}
}

// TODO: Allow impl for `impl SignalGet<Value: WithDynText>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: WithDynText + 'static];
	[T, F] [Derived<T, F> where T: WithDynText + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: WithDynText + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Deref<Target: WithDynText>>];
	[T] [QuerySignal<T> where T: QueryParse<Value: WithDynText>]
)]
impl<Generics> WithDynText for Ty {
	fn with_text<F2, O>(&self, f: F2) -> O
	where
		F2: FnOnce(Option<&str>) -> O,
	{
		self.with(|text| text.with_text(f))
	}
}
