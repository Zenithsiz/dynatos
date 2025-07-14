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
#[expect(
	clippy::partial_pub_fields,
	reason = "TODO: Make these private and hand out references"
)]
pub struct World {
	/// "raw" mode ref count
	raw_ref_count: Cell<usize>,

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
			raw_ref_count: Cell::new(0),
			dep_graph:     DepGraph::new(),
			effect_stack:  EffectStack::new(),
			run_queue:     RunQueue::new(),
		}
	}

	/// Returns if in "raw" mode
	pub const fn is_raw(&self) -> bool {
		self.raw_ref_count.get() > 0
	}

	/// Enters "raw" mode
	pub fn set_raw(&self) -> RawGuard {
		self.raw_ref_count.update(|count| count + 1);
		RawGuard(())
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
