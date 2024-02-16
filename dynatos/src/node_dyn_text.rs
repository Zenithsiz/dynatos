//! Node reactive text

// Imports
use {
	crate::ObjectAttachEffect,
	dynatos_reactive::{Derived, Effect, Signal, SignalWith},
	dynatos_util::{TryOrReturnExt, WeakRef},
};

/// Extension trait to add reactive text to a node
#[extend::ext(name = NodeDynText)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
	/// Adds dynamic text to this node
	fn dyn_text<U>(&self, text: U)
	where
		U: WithDynText + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		let node = WeakRef::new(self.as_ref());
		let text_effect = Effect::try_new(move || {
			// Try to get the node
			let node = node.get().or_return()?;

			// And set the text content
			text.with_text(|text| node.set_text_content(text));
		})
		.or_return()?;

		// Then set it
		self.as_ref().attach_effect(text_effect);
	}

	/// Adds dynamic text to this node.
	///
	/// Returns the node, for chaining
	fn with_dyn_text<U>(self, text: U) -> Self
	where
		U: WithDynText + 'static,
	{
		self.dyn_text(text);
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

#[duplicate::duplicate_item(
	Ty;
	[&'static str];
	[String];
)]
impl WithDynText for Option<Ty> {
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		match self {
			Some(s) => f(Some(s)),
			None => f(None),
		}
	}
}

// TODO: Allow impl for `impl SignalGet<Value: WithDynText>`
#[duplicate::duplicate_item(
	Sig;
	[Signal];
	[Derived];
)]
impl<T> WithDynText for Sig<T>
where
	T: WithDynText,
{
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		self.with(|text| text.with_text(f))
	}
}
