//! Dependency graph

// Imports
#[cfg(debug_assertions)]
use core::panic::Location;
use {
	crate::{Effect, EffectRun, Trigger, WeakEffect, WeakTrigger},
	core::cell::{LazyCell, RefCell},
	petgraph::prelude::{NodeIndex, StableGraph},
	std::collections::HashMap,
};

/// Dependency graph
#[thread_local]
static DEP_GRAPH: LazyCell<RefCell<DepGraph>> = LazyCell::new(|| RefCell::new(DepGraph::new()));

/// Effect dependency info
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub struct EffectDepInfo {
	/// Location this dependency was gathered
	#[cfg(debug_assertions)]
	pub gathered_loc: &'static Location<'static>,
}

/// Effect subscriber info
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub struct EffectSubInfo {
	/// Location this subscriber was executed
	#[cfg(debug_assertions)]
	pub exec_loc: &'static Location<'static>,
}

/// Graph node
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
enum Node {
	/// Trigger
	Trigger(WeakTrigger),

	/// Effect
	Effect(WeakEffect),
}

/// Graph edge
// TODO: Make this a ZST in release mode?
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
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
			#[cfg(debug_assertions)]
			gathered_loc:                          Location::caller(),
		})
	}

	/// Creates an effect subscriber edge
	pub const fn effect_sub(#[cfg(debug_assertions)] caller_loc: &'static Location<'static>) -> Self {
		Self::EffectSub(EffectSubInfo {
			#[cfg(debug_assertions)]
			exec_loc:                          caller_loc,
		})
	}
}

/// Dependency graph
#[derive(Clone, Debug)]
struct DepGraph {
	/// Nodes
	nodes: HashMap<Node, NodeIndex>,

	/// Graph
	graph: StableGraph<Node, Edge>,
}

impl DepGraph {
	/// Creates a new dependency graph
	#[must_use]
	pub fn new() -> Self {
		Self {
			nodes: HashMap::new(),
			graph: StableGraph::new(),
		}
	}

	/// Gets the idx of a node, or creates it
	pub fn get_or_insert_node(&mut self, node: Node) -> NodeIndex {
		*self
			.nodes
			.entry(node)
			.or_insert_with_key(|node| self.graph.add_node(node.clone()))
	}
}

/// Clears an effect's dependencies and subscribers
pub fn clear_effect<F: ?Sized + EffectRun>(effect: &Effect<F>) {
	let mut dep_graph = DEP_GRAPH.borrow_mut();
	let Some(&effect_idx) = dep_graph.nodes.get(&Node::Effect(effect.downgrade().unsize())) else {
		return;
	};

	let mut deps = dep_graph.graph.neighbors_undirected(effect_idx).detach();
	while let Some(edge) = deps.next_edge(&dep_graph.graph) {
		dep_graph.graph.remove_edge(edge);
	}
}

/// Uses all subscribers of a trigger
pub fn with_trigger_subs<F>(trigger: WeakTrigger, mut f: F)
where
	F: FnMut(&WeakEffect, Vec<EffectDepInfo>),
{
	let dep_graph = DEP_GRAPH.borrow();
	let Some(&trigger_idx) = dep_graph.nodes.get(&Node::Trigger(trigger)) else {
		return;
	};

	for effect_idx in dep_graph
		.graph
		.neighbors_directed(trigger_idx, petgraph::Direction::Outgoing)
	{
		let effect = match &dep_graph.graph[effect_idx] {
			Node::Trigger(_) => unreachable!("Trigger had an outgoing edge to another trigger"),
			Node::Effect(effect) => effect,
		};

		let effect_info = dep_graph
			.graph
			.edges_connecting(trigger_idx, effect_idx)
			.map(|edge| match edge.weight() {
				Edge::EffectDep(dep_info) => dep_info.clone(),
				Edge::EffectSub(_) => unreachable!("Trigger has an outgoing edge with effect subscriber info"),
			})
			.collect();

		f(effect, effect_info);
	}
}

/// Adds an effect dependency
#[track_caller]
pub fn add_effect_dep(effect: &Effect, trigger: &Trigger) {
	#[cfg(debug_assertions)]
	tracing::trace!(
		"Adding effect dependency\nEffect  : {}\nTrigger : {}\nGathered: {}",
		effect.defined_loc(),
		trigger.defined_loc(),
		Location::caller(),
	);

	let mut dep_graph = DEP_GRAPH.borrow_mut();

	let effect_idx = dep_graph.get_or_insert_node(Node::Effect(effect.downgrade()));
	let trigger_idx = dep_graph.get_or_insert_node(Node::Trigger(trigger.downgrade()));

	dep_graph.graph.add_edge(trigger_idx, effect_idx, Edge::effect_dep());
}

/// Adds an effect subscriber
pub fn add_effect_sub(
	effect: &Effect,
	trigger: &Trigger,
	#[cfg(debug_assertions)] caller_loc: &'static Location<'static>,
) {
	#[cfg(debug_assertions)]
	tracing::trace!(
		"Adding effect subscriber\nEffect  : {}\nTrigger : {}\nExecuted: {}",
		effect.defined_loc(),
		trigger.defined_loc(),
		caller_loc,
	);

	let mut dep_graph = DEP_GRAPH.borrow_mut();

	let effect_idx = dep_graph.get_or_insert_node(Node::Effect(effect.downgrade()));
	let trigger_idx = dep_graph.get_or_insert_node(Node::Trigger(trigger.downgrade()));

	dep_graph.graph.add_edge(
		effect_idx,
		trigger_idx,
		Edge::effect_sub(
			#[cfg(debug_assertions)]
			caller_loc,
		),
	);
}

/// Exports the dependency graph as a dot graph.
#[cfg(debug_assertions)]
pub fn export_dot() -> String {
	let dep_graph = &DEP_GRAPH.borrow();
	let graph = dep_graph.graph.map(
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
