//! Node reactive children

// Imports
use {
	crate::ObjectAttachEffect,
	core::cell::RefCell,
	dynatos_html::html,
	dynatos_reactive::Effect,
	dynatos_util::{TryOrReturnExt, WeakRef},
	wasm_bindgen::JsCast,
};

/// Extension trait to add a reactive child to an node
#[extend::ext(name = NodeDynChild)]
pub impl web_sys::Node {
	/// Adds a dynamic child to this node
	fn add_dyn_child<C>(&self, child: C)
	where
		C: ToDynNode + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		// Note: We have an empty `<template>` so that we can track the position
		//       of the node, in case of `f` returning `None`.
		// TODO: Find a better solution for when `f` returns `None` that doesn't involve
		//       adding an element to the dom?
		let node = WeakRef::new(self);
		let prev_child = RefCell::new(None::<web_sys::Node>);
		let empty_child = web_sys::Node::from(html::template());
		let child_effect = Effect::try_new(move || {
			// Try to get the node
			let node = node.get().or_return()?;

			// Get the new child
			let new_child = child.to_node();

			// Check if someone's messed with our previous child
			// TODO: At this point should we give up, since we lost the position?
			//       The behavior of trying again might be worse.
			let mut prev_child = prev_child.borrow_mut();
			if let Some(child) = &*prev_child &&
				!node.contains(Some(child))
			{
				tracing::warn!("Reactive child was removed externally, re-inserting");
				*prev_child = None;
			}

			// Then check if we need to substitute in the empty child
			let new_child = match new_child {
				// If the new child is the same as the old one, we can return
				Some(child) if prev_child.as_ref() == Some(&child) => return,

				// Otherwise, if this is a duplicate node, warn and use an empty child
				// Note: The typical browser behavior would be to remove the previous
				//       child, then add ours. Unfortunately, removing other nodes might
				//       cause another dyn child to panic due to it's previous node being
				//       missing.
				Some(child) if node.contains(Some(&child)) => {
					tracing::warn!("Attempted to add a reactive node multiple times");
					empty_child.clone()
				},

				// Otherwise, use the new child
				Some(child) => child,

				// Finally, if no child was given, use the empty child
				None => empty_child.clone(),
			};

			// Then update the node
			match &mut *prev_child {
				// If we already have a node, replace it
				Some(prev_child) => node
					.replace_child(&new_child, prev_child)
					.expect("Unable to replace reactive child"),

				// Otherwise, we're running for the first time, so append the child
				None => node.append_child(&new_child).expect("Unable to append reactive child"),
			};

			*prev_child = Some(new_child);
		})
		.or_return()?;

		// Then set it
		self.attach_effect(child_effect);
	}
}

/// Extension trait to add a reactive child to an node
#[extend::ext(name = NodeWithDynChild)]
pub impl<N> N
where
	N: AsRef<web_sys::Node>,
{
	/// Adds a dynamic child to this node.
	///
	/// Returns the node, for chaining
	fn with_dyn_child<C>(self, child: C) -> Self
	where
		C: ToDynNode + 'static,
	{
		self.as_ref().add_dyn_child(child);
		self
	}
}

/// Trait for values accepted by [`NodeDynChild`].
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `impl Fn() -> Option<N>`
/// - `N`
/// - `Option<N>`
/// Where `N` is a node type.
pub trait ToDynNode {
	/// Retrieves / Computes the inner node
	fn to_node(&self) -> Option<web_sys::Node>;
}

impl<F, N> ToDynNode for F
where
	F: Fn() -> N,
	N: ToDynNode,
{
	fn to_node(&self) -> Option<web_sys::Node> {
		self().to_node()
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
impl ToDynNode for Ty {
	fn to_node(&self) -> Option<web_sys::Node> {
		let node = self.dyn_ref::<web_sys::Node>().expect("Unable to cast to element");
		Some(node.clone())
	}
}

impl<N> ToDynNode for Option<N>
where
	N: ToDynNode,
{
	fn to_node(&self) -> Option<web_sys::Node> {
		self.as_ref().and_then(N::to_node)
	}
}
