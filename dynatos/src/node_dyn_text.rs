//! Node reactive text

// Imports
use {
	crate::ObjectAttachEffect,
	core::ops::Deref,
	dynatos_html::WeakRef,
	dynatos_reactive::{derived::DerivedRun, Derived, Effect, Memo, Signal, SignalWith, WithDefault},
	dynatos_router::{query_signal::QueryParse, QuerySignal},
	dynatos_util::TryOrReturnExt,
};

/// Extension trait to add reactive text to a node
#[extend::ext(name = NodeDynText)]
pub impl web_sys::Node {
	/// Adds dynamic text to this node
	#[track_caller]
	fn set_dyn_text<T>(&self, text: T)
	where
		T: WithDynText + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		let node = WeakRef::new(self);
		let text_effect = Effect::try_new(move || {
			// Try to get the node
			let node = node.get().or_return()?;

			// And set the text content
			text.with_text(|text| node.set_text_content(text));
		})
		.or_return()?;

		// Then set it
		self.attach_effect(text_effect);
	}
}

/// Extension trait to add reactive text to a node
#[extend::ext(name = NodeWithDynText)]
pub impl<N> N
where
	N: AsRef<web_sys::Node>,
{
	/// Adds dynamic text to this node.
	///
	/// Returns the node, for chaining
	#[track_caller]
	fn with_dyn_text<T>(self, text: T) -> Self
	where
		T: WithDynText + 'static,
	{
		self.as_ref().set_dyn_text(text);
		self
	}
}

/// Trait for values accepted by [`NodeDynText`].
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
