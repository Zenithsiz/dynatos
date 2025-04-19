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
	type ContextStack<T: ?Sized>: ContextStack<T, Self>;
}

impl ContextWorld for WorldThreadLocal {
	type ContextStack<T: ?Sized> = ContextStackThreadLocal<T>;
}
impl ContextWorld for WorldGlobal {
	type ContextStack<T: ?Sized> = ContextStackGlobal<T>;
}

/// Any type for the world's context stack
pub type Any<T: ?Sized, W: ContextWorld> = <W::ContextStack<T> as ContextStack<T, W>>::Any;

/// Handle bounds type for the world's context stack
pub type HandleBounds<T: ?Sized, W: ContextWorld> = <W::ContextStack<T> as ContextStack<T, W>>::HandleBounds;
