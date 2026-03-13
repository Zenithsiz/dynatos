//! Node reactive children

// Imports
use {
	crate::ObjectAttachEffect,
	core::{
		cell::{LazyCell, RefCell},
		ops::Deref,
	},
	dynatos_html::html,
	dynatos_reactive::{Derived, Effect, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun},
	dynatos_util::TryOrReturnExt,
	js_sys::WeakRef,
	std::sync::LazyLock,
	wasm_bindgen::JsCast,
};

/// Extension trait to add reactive children to an node
#[extend::ext(name = NodeDynChildren)]
pub impl<N> N
where
	N: AsRef<web_sys::Node>,
{
	/// Adds dynamic children to this node
	#[track_caller]
	fn add_dyn_children<C>(&self, children: C)
	where
		C: WithDynNodes + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		// Note: We have an empty `<template>` so that we can track the position
		//       of the node, in case of `f` returning `None`.
		// TODO: Find a better solution than using an empty `<template>` element?
		let node = WeakRef::new(self.as_ref());
		let prev_children = RefCell::new(vec![]);
		let empty_child = web_sys::Node::from(html::template());
		let child_effect = Effect::try_new(move || {
			// Try to get the node
			let node = node.deref().or_return()?;

			// Replaces a previous child with anew child at an index.
			// If no previous child existed at this position, adds it.
			let replace_prev_child = |prev_children: &mut Vec<_>, idx: usize, new_node| match prev_children.get_mut(idx)
			{
				Some(prev_node) => {
					node.replace_child(&new_node, prev_node)
						.expect("Unable to replace reactive child");

					*prev_node = new_node;
				},
				None => {
					let after_last_existing = prev_children.last().and_then(web_sys::Node::next_sibling);

					node.insert_before(&new_node, after_last_existing.as_ref())
						.expect("Unable to add reactive child");

					prev_children.push(new_node);
				},
			};

			// Add/replace all new children
			let mut idx = 0;
			let mut prev_children = prev_children.borrow_mut();
			children.with_children(|new_node| {
				replace_prev_child(&mut prev_children, idx, new_node);
				idx += 1;
			});

			// Then remove any leftovers (except for the very first node)
			for prev_node in prev_children.drain(idx.max(1)..) {
				node.remove_child(&prev_node).expect("Unable to remove reactive child");
			}

			// If we were going to end up empty, replace the first child
			// with an empty child to keep our position instead.
			if idx == 0 {
				replace_prev_child(&mut prev_children, 0, empty_child.clone());
			}
		})
		.or_return()?;

		// Then set it
		self.as_ref().attach_effect(child_effect);
	}
}

/// Extension trait to add reactive children to an node
#[extend::ext(name = NodeWithDynChildren)]
pub impl<N> N
where
	N: AsRef<web_sys::Node>,
{
	/// Adds dynamic children to this node.
	///
	/// Returns the node, for chaining
	#[track_caller]
	fn with_dyn_children<C>(self, children: C) -> Self
	where
		C: WithDynNodes + 'static,
	{
		self.add_dyn_children(children);
		self
	}
}

/// Trait for values accepted by [`NodeDynChildren`].
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `web_sys::{Node, Element, HtmlElement}`
/// - `Option<N>`
/// - `Vec<N>`, `[N; _]`, `[N]`
/// - [`Signal`], [`Derived`], [`Memo`], [`WithDefault`]
/// - `LazyCell<N, impl Fn() -> N>`
/// - `!`
///
/// Where `N` is any of the types above.
pub trait WithDynNodes {
	/// Calls `f` with all nodes.
	fn with_children(&self, f: impl FnMut(web_sys::Node));
}

impl<F, N> WithDynNodes for F
where
	F: Fn() -> N,
	N: WithDynNodes,
{
	fn with_children(&self, f: impl FnMut(web_sys::Node)) {
		self().with_children(f);
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
	[web_sys::HtmlElement];
)]
impl WithDynNodes for Ty {
	fn with_children(&self, mut f: impl FnMut(web_sys::Node)) {
		let node = self.dyn_ref::<web_sys::Node>().expect("Unable to cast to element");
		f(node.clone());
	}
}

impl<N> WithDynNodes for Option<N>
where
	N: WithDynNodes,
{
	fn with_children(&self, f: impl FnMut(web_sys::Node)) {
		if let Some(children) = self {
			children.with_children(f);
		}
	}
}

impl<N> WithDynNodes for Vec<N>
where
	N: WithDynNodes,
{
	fn with_children(&self, f: impl FnMut(web_sys::Node)) {
		(**self).with_children(f);
	}
}

impl<Node, const N: usize> WithDynNodes for [Node; N]
where
	Node: WithDynNodes,
{
	fn with_children(&self, f: impl FnMut(web_sys::Node)) {
		self.as_slice().with_children(f);
	}
}

impl<N> WithDynNodes for [N]
where
	N: WithDynNodes,
{
	fn with_children(&self, mut f: impl FnMut(web_sys::Node)) {
		for children in self {
			children.with_children(&mut f);
		}
	}
}

// TODO: Allow impl for `impl SignalWith<Value: WithDynNodes>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: WithDynNodes + 'static];
	[T, F] [Derived<T, F> where T: WithDynNodes + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: WithDynNodes + 'static, F: ?Sized + 'static];
	[S, T] [WithDefault<S, T> where Self: for<'a> SignalWith<Value<'a>: Deref<Target: WithDynNodes>>];
)]
impl<Generics> WithDynNodes for Ty {
	fn with_children(&self, f: impl FnMut(web_sys::Node)) {
		#[allow(
			clippy::allow_attributes,
			clippy::redundant_closure_for_method_calls,
			reason = "In some branches it isn't redundant"
		)]
		self.with(|value| value.with_children(f));
	}
}

impl<N, F> WithDynNodes for LazyCell<N, F>
where
	N: WithDynNodes,
	F: FnOnce() -> N,
{
	fn with_children(&self, f: impl FnMut(web_sys::Node)) {
		(**self).with_children(f);
	}
}

impl<N, F> WithDynNodes for LazyLock<N, F>
where
	N: WithDynNodes,
	F: FnOnce() -> N,
{
	fn with_children(&self, f: impl FnMut(web_sys::Node)) {
		(**self).with_children(f);
	}
}

impl WithDynNodes for ! {
	fn with_children(&self, _f: impl FnMut(web_sys::Node)) {}
}
