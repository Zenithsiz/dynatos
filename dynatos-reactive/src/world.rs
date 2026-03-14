//! World

// Imports
use {
	crate::{dep_graph::DepGraph, effect_stack::EffectStack, run_queue::RunQueue},
	core::{
		cell::{Cell, LazyCell},
		ops::{Index, IndexMut},
	},
};

/// Default world
#[thread_local]
pub static WORLD: LazyCell<World> = LazyCell::new(World::new);

/// World
#[derive(Debug)]
pub struct World {
	/// Modes
	modes: WorldModesData,

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
			modes:        WorldModesData::default(),
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

	/// Returns if in a mode
	pub fn is_in_mode(&self, mode: WorldMode) -> bool {
		self.modes[mode].ref_count.get() > 0
	}

	/// Enters a mode
	pub fn enter_mode(&self, mode: WorldMode) -> WorldModeGuard {
		self.modes[mode].ref_count.update(|count| count + 1);
		WorldModeGuard(mode)
	}
}

#[coverage(off)]
impl Default for World {
	fn default() -> Self {
		Self::new()
	}
}

/// Mode data
#[derive(Clone, Default, Debug)]
struct WorldModeData {
	ref_count: Cell<usize>,
}

/// Guard type for entering and exiting a mode
pub struct WorldModeGuard(WorldMode);

impl Drop for WorldModeGuard {
	fn drop(&mut self) {
		WORLD.modes[self.0].ref_count.update(|count| count - 1);
	}
}

macro decl_modes(
	$WorldModesData:ident;
	$WorldMode:ident;

	$(
		$( #[$meta:meta] )*
		$Name:ident($field:ident)
	),* $(,)?
) {
	/// Modes
	#[derive(PartialEq, Eq, Clone, Copy, Debug)]
	pub enum $WorldMode {
		$(
			$Name,
		)*
	}

	/// Modes data
	#[derive(Clone, Default, Debug)]
	struct $WorldModesData {
		$(
			$field: WorldModeData,
		)*
	}

	impl Index<$WorldMode> for $WorldModesData {
		type Output = WorldModeData;

		fn index(&self, mode: $WorldMode) -> &Self::Output {
			match mode {
				$(
					$WorldMode::$Name => &self.$field,
				)*
			}
		}
	}

	impl IndexMut<$WorldMode> for $WorldModesData {
		fn index_mut(&mut self, mode: $WorldMode) -> &mut Self::Output {
			match mode {
				$(
					$WorldMode::$Name => &mut self.$field,
				)*
			}
		}
	}
}

decl_modes! {
	WorldModesData;
	WorldMode;

	/// "raw" mode
	Raw(raw),

	/// "unloaded" mode
	Unloaded(unloaded),
}
