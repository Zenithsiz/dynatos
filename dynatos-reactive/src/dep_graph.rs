//! Dependency graph

// Imports
use {
	crate::{loc::Loc, Effect, EffectRun, Trigger, WeakEffect, WeakTrigger},
	core::cell::RefCell,
	petgraph::prelude::{NodeIndex, StableGraph},
	std::{collections::HashMap, error::Error as StdError},
};

/// Effect dependency info
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub struct EffectDepInfo {
	/// Location this dependency was gathered
	pub gathered_loc: Loc,
}

/// Effect subscriber info
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub struct EffectSubInfo {
	/// Location this subscriber was executed
	pub exec_loc: Loc,
}

/// Graph node
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
#[derive(derive_more::From, derive_more::TryInto)]
enum Node {
	/// Trigger
	Trigger(WeakTrigger),

	/// Effect
	Effect(WeakEffect),
}

/// Graph edge
// TODO: Make this a ZST in release mode?
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
#[derive(derive_more::From, derive_more::TryInto)]
enum Edge {
	/// Effect dependency
	EffectDep(EffectDepInfo),

	/// Effect subscriber
	EffectSub(EffectSubInfo),
}

impl Edge {
	/// Creates an effect dependency edge
	#[track_caller]
	pub const fn effect_dep() -> Self {
		Self::EffectDep(EffectDepInfo {
			gathered_loc: Loc::caller(),
		})
	}

	/// Creates an effect subscriber edge
	pub const fn effect_sub(exec_loc: Loc) -> Self {
		Self::EffectSub(EffectSubInfo { exec_loc })
	}
}

/// Inner
#[derive(Clone, Debug)]
struct Inner {
	/// Nodes
	nodes: HashMap<Node, NodeIndex>,

	/// Graph
	graph: StableGraph<Node, Edge>,
}

/// Dependency graph
#[derive(Clone, Debug)]
pub struct DepGraph {
	/// Inner
	inner: RefCell<Inner>,
}

impl DepGraph {
	/// Creates a new dependency graph
	#[must_use]
	pub fn new() -> Self {
		Self {
			inner: RefCell::new(Inner {
				nodes: HashMap::new(),
				graph: StableGraph::new(),
			}),
		}
	}

	/// Gets the idx of a node, or creates it
	fn get_or_insert_node(&self, node: Node) -> NodeIndex {
		let mut inner = self.inner.borrow_mut();
		let inner = &mut *inner;

		*inner
			.nodes
			.entry(node)
			.or_insert_with_key(|node| inner.graph.add_node(node.clone()))
	}

	/// Clears an effect's dependencies and subscribers
	pub fn clear_effect<F: ?Sized + EffectRun>(&self, effect: &Effect<F>) {
		let mut inner = self.inner.borrow_mut();
		let Some(&effect_idx) = inner.nodes.get(&Node::Effect(effect.downgrade().unsize())) else {
			return;
		};

		let mut deps = inner.graph.neighbors_undirected(effect_idx).detach();
		while let Some(edge) = deps.next_edge(&inner.graph) {
			inner.graph.remove_edge(edge);
		}
	}

	/// Uses all dependencies/subscribers of a trigger/effect
	pub fn with<W>(&self, start: W::StartNode, mut f: impl WithFn<W>)
	where
		W: With,
	{
		let mut inner = self.inner.borrow();
		let Some(&trigger_idx) = inner.nodes.get(&start.into()) else {
			return;
		};

		// TODO: If we have multiple edges to a neighbor, will this go through them once or
		//       once for each edge?
		let mut neighbors = inner.graph.neighbors_directed(trigger_idx, W::DIR).detach();
		loop {
			let Some(effect_idx) = neighbors.next_node(&inner.graph) else {
				break;
			};

			let end = TryFrom::try_from(inner.graph[effect_idx].clone())
				.expect("Trigger/Effect had an edge to another trigger/effect");

			let effect_info = inner
				.graph
				.edges_connecting(trigger_idx, effect_idx)
				.map(|edge| TryFrom::try_from(edge.weight().clone()).expect("Trigger/effect had the wrong edge type"))
				.collect();

			drop(inner);
			f(end, effect_info);
			inner = self.inner.borrow();
		}
	}

	/// Uses all subscribers of a trigger
	pub fn with_trigger_subs(&self, trigger: WeakTrigger, f: impl WithFn<WithTriggerSubs>) {
		self.with::<WithTriggerSubs>(trigger, f);
	}

	/// Uses all dependencies of a trigger
	pub fn with_trigger_deps(&self, trigger: WeakTrigger, f: impl WithFn<WithTriggerDeps>) {
		self.with::<WithTriggerDeps>(trigger, f);
	}

	/// Uses all subscribers of an effect
	pub fn with_effect_subs(&self, effect: WeakEffect, f: impl WithFn<WithEffectSubs>) {
		self.with::<WithEffectSubs>(effect, f);
	}

	/// Uses all dependencies of an effect
	pub fn with_effect_deps(&self, effect: WeakEffect, f: impl WithFn<WithEffectDeps>) {
		self.with::<WithEffectDeps>(effect, f);
	}

	/// Adds an effect dependency
	#[track_caller]
	pub fn add_effect_dep(&self, effect: &Effect, trigger: &Trigger) {
		tracing::trace!(
			"Adding effect dependency\nEffect  : {}\nTrigger : {}\nGathered: {}",
			effect.defined_loc(),
			trigger.defined_loc(),
			Loc::caller(),
		);

		let effect_idx = self.get_or_insert_node(Node::Effect(effect.downgrade()));
		let trigger_idx = self.get_or_insert_node(Node::Trigger(trigger.downgrade()));

		self.inner
			.borrow_mut()
			.graph
			.add_edge(trigger_idx, effect_idx, Edge::effect_dep());
	}

	/// Adds an effect subscriber
	pub fn add_effect_sub(&self, effect: &Effect, trigger: &Trigger, caller_loc: Loc) {
		tracing::trace!(
			"Adding effect subscriber\nEffect  : {}\nTrigger : {}\nExecuted: {}",
			effect.defined_loc(),
			trigger.defined_loc(),
			caller_loc,
		);

		let effect_idx = self.get_or_insert_node(Node::Effect(effect.downgrade()));
		let trigger_idx = self.get_or_insert_node(Node::Trigger(trigger.downgrade()));

		self.inner
			.borrow_mut()
			.graph
			.add_edge(effect_idx, trigger_idx, Edge::effect_sub(caller_loc));
	}

	/// Exports the dependency graph as a dot graph.
	pub fn export_dot(&self) -> String {
		let inner = &self.inner.borrow();
		let graph = inner.graph.map(
			|_node_idx, node| match node {
				Node::Trigger(trigger) => match trigger.upgrade() {
					Some(trigger) => format!("Trigger({})", trigger.defined_loc()),
					None => "Trigger(<dropped>)".to_owned(),
				},
				Node::Effect(effect) => match effect.upgrade() {
					Some(effect) => format!("Effect({})", effect.defined_loc()),
					None => "Effect(<dropped>)".to_owned(),
				},
			},
			|_edge_idx, edge| match edge {
				Edge::EffectDep(info) => format!("Gather({})", info.gathered_loc),
				Edge::EffectSub(info) => format!("Exec({})", info.exec_loc),
			},
		);

		petgraph::dot::Dot::new(&graph).to_string()
	}
}

#[coverage(off)]
impl Default for DepGraph {
	fn default() -> Self {
		Self::new()
	}
}

/// Function trait for [`with`] and friends.
pub trait WithFn<W: With> = FnMut(W::EndNode, Vec<W::Info>);

/// Dep graph with
#[expect(private_bounds, reason = "It's a sealed trait")]
pub trait With {
	/// Start node
	type StartNode: Into<Node>;

	/// End node
	type EndNode: TryFrom<Node, Error: StdError>;

	/// Info type
	type Info: TryFrom<Edge, Error: StdError>;

	/// Direction
	const DIR: petgraph::Direction;
}

pub struct WithTriggerSubs;
impl With for WithTriggerSubs {
	type EndNode = WeakEffect;
	type Info = EffectDepInfo;
	type StartNode = WeakTrigger;

	const DIR: petgraph::Direction = petgraph::Direction::Outgoing;
}

pub struct WithTriggerDeps;
impl With for WithTriggerDeps {
	type EndNode = WeakEffect;
	type Info = EffectSubInfo;
	type StartNode = WeakTrigger;

	const DIR: petgraph::Direction = petgraph::Direction::Incoming;
}

pub struct WithEffectDeps;
impl With for WithEffectDeps {
	type EndNode = WeakTrigger;
	type Info = EffectDepInfo;
	type StartNode = WeakEffect;

	const DIR: petgraph::Direction = petgraph::Direction::Incoming;
}

pub struct WithEffectSubs;
impl With for WithEffectSubs {
	type EndNode = WeakTrigger;
	type Info = EffectSubInfo;
	type StartNode = WeakEffect;

	const DIR: petgraph::Direction = petgraph::Direction::Outgoing;
}
