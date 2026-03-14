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
	pub fn has_tag(&self, tag: WorldTag) -> bool {
		self.tags[tag].ref_count.get() > 0
	}

	/// Adds a tag to the world until the guard is dropped.
	// TODO: Specify what happens when recursive tags are added & dropped.
	pub fn add_tag(&self, tag: WorldTag) -> WorldTagGuard {
		self.tags[tag].ref_count.update(|count| count + 1);
		WorldTagGuard(tag)
	}
}

#[coverage(off)]
impl Default for World {
	fn default() -> Self {
		Self::new()
	}
}

/// Tag data
#[derive(Clone, Default, Debug)]
struct WorldTagData {
	ref_count: Cell<usize>,
}

/// Guard type for entering and exiting a tag
pub struct WorldTagGuard(WorldTag);

impl Drop for WorldTagGuard {
	fn drop(&mut self) {
		WORLD.tags[self.0].ref_count.update(|count| count - 1);
	}
}

macro decl_tags(
	$WorldTagsData:ident;
	$WorldTag:ident;

	$(
		$( #[$meta:meta] )*
		$Name:ident($field:ident)
	),* $(,)?
) {
	/// Tags
	#[derive(PartialEq, Eq, Clone, Copy, Debug)]
	pub enum $WorldTag {
		$(
			$Name,
		)*
	}

	/// Tags data
	#[derive(Clone, Default, Debug)]
	struct $WorldTagsData {
		$(
			$field: WorldTagData,
		)*
	}

	impl Index<$WorldTag> for $WorldTagsData {
		type Output = WorldTagData;

		fn index(&self, tag: $WorldTag) -> &Self::Output {
			match tag {
				$(
					$WorldTag::$Name => &self.$field,
				)*
			}
		}
	}

	impl IndexMut<$WorldTag> for $WorldTagsData {
		fn index_mut(&mut self, tag: $WorldTag) -> &mut Self::Output {
			match tag {
				$(
					$WorldTag::$Name => &mut self.$field,
				)*
			}
		}
	}
}

decl_tags! {
	WorldTagsData;
	WorldTag;

	/// "raw" tag
	Raw(raw),

	/// "unloaded" tag
	Unloaded(unloaded),
}
