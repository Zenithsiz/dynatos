//! Node reactive text

// Imports
use {
	crate::ObjectAttachEffect,
	dynatos_reactive::{Derived, Effect, Signal, SignalWith, WithDefault},
	dynatos_router::QuerySignal,
	dynatos_util::{TryOrReturnExt, WeakRef},
};

/// Extension trait to add reactive text to a node
#[extend::ext(name = NodeDynText)]
pub impl web_sys::Node {
	/// Adds dynamic text to this node
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
/// - `impl Fn() -> Option<N>`
/// - `N`
/// - `Option<N>`
/// Where `N` is a text type.
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

#[duplicate::duplicate_item(
	Ty;
	[str];
	[&'static str];
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
	[T, F] [Derived<T, F> where T: WithDynText + 'static, F: ?Sized];
	[T] [QuerySignal<T> where T: WithDynText + 'static];
	[S, T] [WithDefault<S, T> where S: for<'a> SignalWith<Value<'a> = &'a Option<T>>, T: WithDynText + 'static];
)]
impl<Generics> WithDynText for Ty {
	fn with_text<F2, O>(&self, f: F2) -> O
	where
		F2: FnOnce(Option<&str>) -> O,
	{
		self.with(|text| text.with_text(f))
	}
}
