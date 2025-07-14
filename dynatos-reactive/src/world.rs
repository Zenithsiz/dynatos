//! World

// Imports
use {
	crate::{dep_graph::DepGraph, effect_stack::EffectStack, run_queue::RunQueue},
	core::cell::{Cell, LazyCell},
};

/// Default world
#[thread_local]
pub static WORLD: LazyCell<World> = LazyCell::new(World::new);

/// World
#[derive(Debug)]
pub struct World {
	/// "raw" mode ref count
	raw_ref_count: Cell<usize>,

	/// "unloaded" mode ref count
	unloaded_ref_count: Cell<usize>,

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
			raw_ref_count:      Cell::new(0),
			unloaded_ref_count: Cell::new(0),
			dep_graph:          DepGraph::new(),
			effect_stack:       EffectStack::new(),
			run_queue:          RunQueue::new(),
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

	/// Returns if in "raw" mode
	pub const fn is_raw(&self) -> bool {
		self.raw_ref_count.get() > 0
	}

	/// Returns if in "unloaded" mode
	pub const fn is_unloaded(&self) -> bool {
		self.unloaded_ref_count.get() > 0
	}

	/// Enters "raw" mode
	pub fn set_raw(&self) -> RawGuard {
		self.raw_ref_count.update(|count| count + 1);
		RawGuard(())
	}

	/// Enters "unloaded" mode
	pub fn set_unloaded(&self) -> UnloadedGuard {
		self.unloaded_ref_count.update(|count| count + 1);
		UnloadedGuard(())
	}
}

#[coverage(off)]
impl Default for World {
	fn default() -> Self {
		Self::new()
	}
}

/// Guard type for entering "raw" mode.
pub struct RawGuard(());

impl Drop for RawGuard {
	fn drop(&mut self) {
		WORLD.raw_ref_count.update(|count| count - 1);
	}
}

/// Guard type for entering "unloaded" mode.
pub struct UnloadedGuard(());

impl Drop for UnloadedGuard {
	fn drop(&mut self) {
		WORLD.unloaded_ref_count.update(|count| count - 1);
	}
}
