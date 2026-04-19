//! Node reactive children

// Imports
use {
	crate::ObjectAttachEffect,
	core::{mem, ops::Deref},
	dynatos_reactive::{Derived, Effect, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun},
	dynatos_sync_types::{IMut, SyncBounds},
	dynatos_util::TryOrReturnExt,
	dynatos_web::{
		DynatosWebCtx,
		html,
		types::{Element, HtmlElement, Node, WeakRef, cfg_ssr_expr},
	},
};

/// Extension trait to add reactive children to an node
#[extend::ext(name = NodeDynChildren)]
pub impl Node {
	/// Adds dynamic children to this node
	#[track_caller]
	fn add_dyn_children<C>(&self, ctx: &DynatosWebCtx, children: C)
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
		let node = WeakRef::new(self);
		let prev_children = IMut::new(vec![]);
		let empty_child = Node::from(html::template(ctx));
		let child_effect = Effect::try_new(move || {
			// Try to get the node
			let node = node.deref().or_return()?;

			// Add/replace all new children
			// TODO: Not collect all children here?
			let mut new_children = vec![];
			children.with_nodes(|new_node| new_children.push(new_node));

			let mut prev_children = prev_children.lock();

			// If we hadn't initialized yet (no previous children) and we have no new children,
			// add our empty child to keep our position
			if prev_children.is_empty() && new_children.is_empty() {
				node.append_child(&empty_child).expect("Unable to add reactive child");
				new_children.push(empty_child.clone());
			}

			// Take all previous nodes we have and start trying to match them against the new ones
			let mut new_nodes = new_children.into_iter();
			for cur_prev_node in mem::take(&mut *prev_children) {
				// Try to find the current previous node we had in the remaining new nodes
				match new_nodes
					.as_slice()
					.iter()
					.position(|new_node| *new_node == cur_prev_node)
				{
					// If we did find it, we know that `new_nodes[..offset]` are new nodes,
					// and `new_nodes[offset]` is an existing node that we can skip.
					Some(offset) => {
						for new_node in (&mut new_nodes).take(offset) {
							node.insert_before(&new_node, Some(&cur_prev_node))
								.expect("Unable to add reactive child");
							self::trace_add_node(&new_node, Some(&cur_prev_node));
							prev_children.push(new_node);
						}

						self::trace_keep_node(&cur_prev_node);
						_ = new_nodes.next();
						prev_children.push(cur_prev_node);
					},

					// If it didn't exist, we can safely remove it.
					None => {
						self::trace_remove_node(&cur_prev_node);
						_ = node
							.remove_child(&cur_prev_node)
							.expect("Unable to remove reactive child");
					},
				}
			}

			for new_node in new_nodes {
				let last_prev_child = prev_children.last().and_then(Node::next_sibling);

				node.insert_before(&new_node, last_prev_child.as_ref())
					.expect("Unable to add reactive child");
				self::trace_add_node(&new_node, last_prev_child.as_ref());
				prev_children.push(new_node);
			}
		})
		.or_return()?;

		// Then set it
		self.attach_effect(child_effect);
	}
}

/// Extension trait to add reactive children to an node
#[extend::ext(name = NodeWithDynChildren)]
pub impl<N> N
where
	N: AsRef<Node>,
{
	/// Adds dynamic children to this node.
	///
	/// Returns the node, for chaining
	#[track_caller]
	fn with_dyn_children<C>(self, ctx: &DynatosWebCtx, children: C) -> Self
	where
		C: WithDynNodes + 'static,
	{
		self.as_ref().add_dyn_children(ctx, children);
		self
	}
}

/// Trait for values accepted by [`NodeDynChildren`].
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `{Node, Element, HtmlElement}`
/// - `Option<N>`
/// - `Vec<N>`, `[N; _]`, `[N]`
/// - [`Signal`], [`Derived`], [`Memo`], [`WithDefault`]
/// - `LazyCell<N, impl Fn() -> N>`
/// - `!`
///
/// Where `N` is any of the types above.
pub trait WithDynNodes: SyncBounds {
	/// Calls `f` with all nodes.
	fn with_nodes(&self, f: impl FnMut(Node));
}

impl<F, N> WithDynNodes for F
where
	F: SyncBounds + Fn() -> N,
	N: WithDynNodes,
{
	fn with_nodes(&self, f: impl FnMut(Node)) {
		self().with_nodes(f);
	}
}

// TODO: Impl for `impl AsRef<Node>` if we can get rid of
//       the conflict with the function impl
#[allow(clippy::allow_attributes, reason = "This only applies in some branches")]
#[allow(clippy::use_self, reason = "We always want to use `Node`, not `Ty`")]
#[duplicate::duplicate_item(
	Ty;
	[Node];
	[Element];
	[HtmlElement];
)]
impl WithDynNodes for Ty {
	fn with_nodes(&self, mut f: impl FnMut(Node)) {
		let node = <Self as AsRef<Node>>::as_ref(self);

		f(node.clone());
	}
}

impl<N> WithDynNodes for Option<N>
where
	N: WithDynNodes,
{
	fn with_nodes(&self, f: impl FnMut(Node)) {
		if let Some(children) = self {
			children.with_nodes(f);
		}
	}
}

impl<N> WithDynNodes for Vec<N>
where
	N: WithDynNodes,
{
	fn with_nodes(&self, f: impl FnMut(Node)) {
		(**self).with_nodes(f);
	}
}

impl<N, const LEN: usize> WithDynNodes for [N; LEN]
where
	N: WithDynNodes,
{
	fn with_nodes(&self, f: impl FnMut(Node)) {
		self.as_slice().with_nodes(f);
	}
}

impl<N> WithDynNodes for [N]
where
	N: WithDynNodes,
{
	fn with_nodes(&self, mut f: impl FnMut(Node)) {
		for children in self {
			children.with_nodes(&mut f);
		}
	}
}

// TODO: Allow impl for `impl SignalWith<Value: WithDynNodes>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: WithDynNodes + 'static];
	[T, F] [Derived<T, F> where T: WithDynNodes + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: WithDynNodes + 'static, F: SyncBounds + ?Sized + 'static];
	[S, T] [WithDefault<S, T> where S: SyncBounds, T: SyncBounds, Self: for<'a> SignalWith<Value<'a>: Deref<Target: WithDynNodes>>];
)]
impl<Generics> WithDynNodes for Ty {
	fn with_nodes(&self, f: impl FnMut(Node)) {
		#[allow(
			clippy::allow_attributes,
			clippy::redundant_closure_for_method_calls,
			reason = "In some branches it isn't redundant"
		)]
		self.with(|value| value.with_nodes(f));
	}
}

#[expect(clippy::absolute_paths, reason = "We want to be explicit due to the `sync` feature")]
impl<N, F> WithDynNodes for core::cell::LazyCell<N, F>
where
	N: WithDynNodes,
	F: FnOnce() -> N,
	Self: SyncBounds,
{
	fn with_nodes(&self, f: impl FnMut(Node)) {
		(**self).with_nodes(f);
	}
}

#[expect(clippy::absolute_paths, reason = "We want to be explicit due to the `sync` feature")]
impl<N, F> WithDynNodes for std::sync::LazyLock<N, F>
where
	N: WithDynNodes,
	F: FnOnce() -> N,
	Self: SyncBounds,
{
	fn with_nodes(&self, f: impl FnMut(Node)) {
		(**self).with_nodes(f);
	}
}

impl WithDynNodes for ! {
	fn with_nodes(&self, _f: impl FnMut(Node)) {}
}

/// Traces the addition of a dynamic node
fn trace_add_node(node: &Node, after: Option<&Node>) {
	cfg_ssr_expr!(
		ssr = tracing::trace!(?node, ?after, "Added new reactive child"),
		csr = match after {
			Some(after) => web_sys::console::debug_3(
				&wasm_bindgen::JsValue::from(
					"dynatos_web_reactive::node_dyn_children: Added new reactive child %o after %o"
				),
				node,
				after,
			),
			None => web_sys::console::debug_2(
				&wasm_bindgen::JsValue::from("dynatos_web_reactive::node_dyn_children: Adding new reactive child %o"),
				node
			),
		}
	);
}

/// Traces the removal of a dynamic node
fn trace_remove_node(node: &Node) {
	cfg_ssr_expr!(
		ssr = tracing::trace!(?node, "Removing reactive child"),
		csr = web_sys::console::debug_2(
			&wasm_bindgen::JsValue::from("dynatos_web_reactive::node_dyn_children: Removing reactive child %o"),
			node
		)
	);
}

/// Traces the keeping of a dynamic node
fn trace_keep_node(node: &Node) {
	cfg_ssr_expr!(
		ssr = tracing::trace!(?node, "Keeping reactive child"),
		csr = web_sys::console::debug_2(
			&wasm_bindgen::JsValue::from("dynatos_web_reactive::node_dyn_children: Keeping reactive child %o"),
			node
		)
	);
}
