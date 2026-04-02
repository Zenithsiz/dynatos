//! World

// Modules
mod tags;

// Exports
pub use self::tags::{WorldTag, WorldTagGuard};

// Imports
use {
	self::tags::{WorldTagState, WorldTagsData},
	crate::{dep_graph::DepGraph, effect_stack::EffectStack, run_queue::RunQueue},
	core::cell::LazyCell,
};

/// Global world
#[thread_local]
pub static GLOBAL_WORLD: LazyCell<GlobalWorld> = LazyCell::new(GlobalWorld::new);

/// Thread-local world
#[thread_local]
pub static THREAD_WORLD: ThreadWorld = ThreadWorld::new();

/// Global world.
#[derive(Debug)]
pub struct GlobalWorld {
	/// Dependency graph
	dep_graph: DepGraph,
}

impl GlobalWorld {
	/// Creates a new world
	#[must_use]
	pub fn new() -> Self {
		Self {
			dep_graph: DepGraph::new(),
		}
	}

	/// Returns the dependency graph
	#[must_use]
	pub const fn dep_graph(&self) -> &DepGraph {
		&self.dep_graph
	}
}

#[coverage(off)]
impl Default for GlobalWorld {
	fn default() -> Self {
		Self::new()
	}
}

/// Per-thread world.
#[derive(Debug)]
pub struct ThreadWorld {
	/// Tags
	tags: WorldTagsData,

	/// Effect stack
	effect_stack: EffectStack,

	/// Run queue
	run_queue: RunQueue,
}

impl ThreadWorld {
	/// Creates a new world
	#[must_use]
	pub const fn new() -> Self {
		Self {
			tags:         WorldTagsData::new(),
			effect_stack: EffectStack::new(),
			run_queue:    RunQueue::new(),
		}
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

	/// Returns if a tag is present and enabled
	pub fn has_tag(&self, tag: WorldTag) -> bool {
		self.tags.get(tag).is_some_and(|tag| tag == WorldTagState::Enabled)
	}

	/// Adds a tag to the world until the guard is dropped.
	// TODO: Specify what happens when recursive tags are added & dropped.
	pub fn add_tag(&self, tag: WorldTag) -> WorldTagGuard {
		self.tags.push(tag, WorldTagState::Enabled)
	}

	/// Removes a tag from the world until the guard is dropped.
	// TODO: Specify what happens when recursive tags are added & dropped.
	pub fn remove_tag(&self, tag: WorldTag) -> WorldTagGuard {
		self.tags.push(tag, WorldTagState::Disabled)
	}
}

#[coverage(off)]
impl Default for ThreadWorld {
	fn default() -> Self {
		Self::new()
	}
}
