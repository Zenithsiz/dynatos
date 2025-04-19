//! World

// Lints
#![expect(
	type_alias_bounds,
	reason = "Although they're not enforced currently, they will be in the future and we want to be explicit already"
)]

// Modules
pub mod context_stack;

// Exports
pub use self::context_stack::{ContextStack, ContextStackGlobal, ContextStackThreadLocal};

// Imports
use dynatos_world::{World, WorldGlobal, WorldThreadLocal};

/// Context world
pub trait ContextWorld: World {
	/// Context stack
	type ContextStack: ContextStack<Self>;
}

impl ContextWorld for WorldThreadLocal {
	type ContextStack = ContextStackThreadLocal;
}
impl ContextWorld for WorldGlobal {
	type ContextStack = ContextStackGlobal;
}

/// Any type for the world's context stack
pub type Any<W: ContextWorld> = <W::ContextStack as ContextStack<W>>::Any;

/// Handle bounds type for the world's context stack
pub type HandleBounds<W: ContextWorld> = <W::ContextStack as ContextStack<W>>::HandleBounds;
