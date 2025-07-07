//! World

// Imports
use {
	crate::{dep_graph::DepGraph, effect_stack::EffectStack, run_queue::RunQueue},
	core::cell::LazyCell,
};

/// Default world
#[thread_local]
pub static WORLD: LazyCell<World> = LazyCell::new(World::new);

/// World
#[derive(Debug)]
pub struct World {
	/// Dependency graph
	pub dep_graph: DepGraph,

	/// Effect stack
	pub effect_stack: EffectStack,

	/// Run queue
	pub run_queue: RunQueue,
}

impl World {
	/// Creates a new world
	#[must_use]
	pub fn new() -> Self {
		Self {
			dep_graph:    DepGraph::new(),
			effect_stack: EffectStack::new(),
			run_queue:    RunQueue::new(),
		}
	}
}

#[coverage(off)]
impl Default for World {
	fn default() -> Self {
		Self::new()
	}
}
