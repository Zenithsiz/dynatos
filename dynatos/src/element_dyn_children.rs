//! Node reactive children

// Imports
use {
	crate::ObjectAttachEffect,
	dynatos_reactive::Effect,
	dynatos_util::{TryOrReturnExt, WeakRef},
};

/// Extension trait to reactively manage all children of an element
#[extend::ext(name = ElementDynChildren)]
pub impl web_sys::Element {
	/// Sets the children of this element
	fn set_dyn_children<C>(&self, children: C)
	where
		C: WithDynNodes + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the element.
		//       Otherwise, the element will be keeping us alive, while we keep
		//       the element alive, causing a leak.
		let element = WeakRef::new(self);
		let cur_children = js_sys::Array::new();
		let child_effect = Effect::try_new(move || {
			// Try to get the element
			let element = element.get().or_return()?;

			// Get the new children
			cur_children.set_length(0);
			children.with_nodes(|child| {
				cur_children.push(child);
			});

			element.replace_children_with_node(&cur_children);
		})
		.or_return()?;

		// Then set it
		self.attach_effect(child_effect);
	}
}

/// Extension trait to reactively manage all children of an element
#[extend::ext(name = ElementWithDynChildren)]
pub impl<N> N
where
	N: AsRef<web_sys::Element>,
{
	/// Sets the children of this element.
	///
	/// Returns the element, for chaining
	fn with_dyn_children<C>(self, children: C) -> Self
	where
		C: WithDynNodes + 'static,
	{
		self.as_ref().set_dyn_children(children);
		self
	}
}

/// Trait for values accepted by [`ElementDynChildren`].
pub trait WithDynNodes {
	/// Executes `f` for each node
	fn with_nodes<F>(&self, f: F)
	where
		F: FnMut(&web_sys::Node);
}

impl<F, N> WithDynNodes for F
where
	F: Fn() -> N,
	N: WithDynNodes + 'static,
{
	fn with_nodes<F2>(&self, f: F2)
	where
		F2: FnMut(&web_sys::Node),
	{
		self().with_nodes(f);
	}
}

// TODO: Impl for `impl AsRef<web_sys::Node>` if we can get rid of
//       the conflict with the function impl
#[allow(clippy::allow_attributes, reason = "This only applies in some branches")]
#[allow(clippy::use_self, reason = "We always want to use `web_sys::Node`, not `Ty`")]
#[duplicate::duplicate_item(
	Ty;
	[web_sys::Node];
	[web_sys::Element];
)]
impl WithDynNodes for Ty {
	fn with_nodes<F>(&self, mut f: F)
	where
		F: FnMut(&web_sys::Node),
	{
		f(self);
	}
}

#[duplicate::duplicate_item(
	Generics Ty;
	[N] [Vec<N>];
	[N] [[N]];
	[N, const SIZE: usize] [[N; SIZE]];
)]
impl<Generics> WithDynNodes for Ty
where
	N: WithDynNodes + 'static,
{
	fn with_nodes<F>(&self, mut f: F)
	where
		F: FnMut(&web_sys::Node),
	{
		for nodes in self {
			nodes.with_nodes(&mut f);
		}
	}
}
