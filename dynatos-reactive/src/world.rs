//! World

// Modules
mod tags;

// Exports
pub use self::tags::{WorldTag, WorldTagGuard};

// Imports
use {
	self::tags::WorldTagsData,
	crate::{dep_graph::DepGraph, effect_stack::EffectStack, run_queue::RunQueue},
	core::cell::LazyCell,
};

/// Default world
#[thread_local]
pub static WORLD: LazyCell<World> = LazyCell::new(World::new);

/// World
#[derive(Debug)]
pub struct World {
	/// Tags
	tags: WorldTagsData,

	/// Dependency graph
	dep_graph: DepGraph,

	/// Effect stack
	effect_stack: EffectStack,

	/// Run queue
	run_queue: RunQueue,
}

impl World {
	/// Creates a new world
	#[must_use]
	pub fn new() -> Self {
		Self {
			tags:         WorldTagsData::default(),
			dep_graph:    DepGraph::new(),
			effect_stack: EffectStack::new(),
			run_queue:    RunQueue::new(),
		}
	}

	/// Returns the dependency graph
	#[must_use]
	pub const fn dep_graph(&self) -> &DepGraph {
		&self.dep_graph
	}

	/// Returns the effect stack
	#[must_use]
	pub const fn effect_stack(&self) -> &EffectStack {
		&self.effect_stack
	}

	/// Returns the run queue
	#[must_use]
	pub const fn run_queue(&self) -> &RunQueue {
		&self.run_queue
	}

	/// Returns if a tag is present
	pub const fn has_tag(&self, tag: WorldTag) -> bool {
		self.tags.has_tag(tag)
	}

	/// Adds a tag to the world until the guard is dropped.
	// TODO: Specify what happens when recursive tags are added & dropped.
	pub fn add_tag(&self, tag: WorldTag) -> WorldTagGuard {
		self.tags.add_tag(tag)
	}
}

#[coverage(off)]
impl Default for World {
	fn default() -> Self {
		Self::new()
	}
}
