//! Node reactive child

// Imports
use {
	crate::{NodeDynChildren, WithDynNodes},
	core::ops::Deref,
	dynatos_reactive::{Derived, Memo, Signal, SignalWith, WithDefault, derived::DerivedRun},
	dynatos_sync_types::SyncBounds,
	dynatos_web::DynatosWebCtx,
	dynatos_web::types::{Element, HtmlElement, Node},
};

/// Extension trait to add a reactive child to an node
#[extend::ext(name = NodeDynChild)]
pub impl Node {
	/// Adds a dynamic child to this node
	#[track_caller]
	fn add_dyn_child<C>(&self, ctx: &DynatosWebCtx, child: C)
	where
		C: WithDynNode + 'static,
	{
		// Delegate to `add_dyn_children`
		struct Wrapper<C>(C);
		impl<C: WithDynNode> WithDynNodes for Wrapper<C> {
			fn with_nodes(&self, f: impl FnMut(Node)) {
				self.0.with_node(f);
			}
		}

		self.add_dyn_children(ctx, Wrapper(child));
	}
}

/// Extension trait to add a reactive child to an node
#[extend::ext(name = NodeWithDynChild)]
pub impl<N> N
where
	N: AsRef<Node>,
{
	/// Adds dynamic a child to this node.
	///
	/// Returns the node, for chaining
	#[track_caller]
	fn with_dyn_child<C>(self, ctx: &DynatosWebCtx, child: C) -> Self
	where
		C: WithDynNode + 'static,
	{
		self.as_ref().add_dyn_child(ctx, child);
		self
	}
}

/// Trait for values accepted by [`NodeDynChild`].
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
pub trait WithDynNode: SyncBounds {
	/// Calls `f` with all nodes.
	fn with_node(&self, f: impl FnMut(Node));
}

impl<F, N> WithDynNode for F
where
	F: SyncBounds + Fn() -> N,
	N: WithDynNode,
{
	fn with_node(&self, f: impl FnMut(Node)) {
		self().with_node(f);
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
impl WithDynNode for Ty {
	fn with_node(&self, mut f: impl FnMut(Node)) {
		let node = <Self as AsRef<Node>>::as_ref(self);

		f(node.clone());
	}
}

impl<N> WithDynNode for Option<N>
where
	N: WithDynNode,
{
	fn with_node(&self, f: impl FnMut(Node)) {
		if let Some(children) = self {
			children.with_node(f);
		}
	}
}

// TODO: Allow impl for `impl SignalWith<Value: WithDynNodes>`
#[duplicate::duplicate_item(
	Generics Ty;
	[T] [Signal<T> where T: WithDynNode + 'static];
	[T, F] [Derived<T, F> where T: WithDynNode + 'static, F: ?Sized + DerivedRun<T> + 'static];
	[T, F] [Memo<T, F> where T: WithDynNode + 'static, F: SyncBounds + ?Sized + 'static];
	[S, T] [WithDefault<S, T> where S: SyncBounds, T: SyncBounds, Self: for<'a> SignalWith<Value<'a>: Deref<Target: WithDynNode>>];
)]
impl<Generics> WithDynNode for Ty {
	fn with_node(&self, f: impl FnMut(Node)) {
		#[allow(
			clippy::allow_attributes,
			clippy::redundant_closure_for_method_calls,
			reason = "In some branches it isn't redundant"
		)]
		self.with(|value| value.with_node(f));
	}
}

#[expect(clippy::absolute_paths, reason = "We want to be explicit due to the `sync` feature")]
impl<N, F> WithDynNode for core::cell::LazyCell<N, F>
where
	N: WithDynNode,
	F: FnOnce() -> N,
	Self: SyncBounds,
{
	fn with_node(&self, f: impl FnMut(Node)) {
		(**self).with_node(f);
	}
}

#[expect(clippy::absolute_paths, reason = "We want to be explicit due to the `sync` feature")]
impl<N, F> WithDynNode for std::sync::LazyLock<N, F>
where
	N: WithDynNode,
	F: FnOnce() -> N,
	Self: SyncBounds,
{
	fn with_node(&self, f: impl FnMut(Node)) {
		(**self).with_node(f);
	}
}

impl WithDynNode for ! {
	fn with_node(&self, _f: impl FnMut(Node)) {}
}
