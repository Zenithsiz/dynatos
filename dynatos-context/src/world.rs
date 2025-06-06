//! World

// Lints
#![expect(
	type_alias_bounds,
	reason = "Although they're not enforced currently, they will be in the future and we want to be explicit already"
)]

// Modules
pub mod context_stack;

// Exports
pub use self::context_stack::{ContextStack, ContextStackGlobal, ContextStackOpaque, ContextStackThreadLocal};

// Imports
use {
	core::any::Any as StdAny,
	dynatos_world::{World, WorldGlobal, WorldThreadLocal},
};

/// Context world
pub trait ContextWorld: World {
	/// Context stack
	type ContextStack<T>: ContextStack<T, Self>;

	/// Opaque context stack
	type ContextStackOpaque: ContextStackOpaque<Self>;
}

impl ContextWorld for WorldThreadLocal {
	type ContextStack<T> = ContextStackThreadLocal<T>;
	type ContextStackOpaque = ContextStackThreadLocal<dyn StdAny>;
}

impl ContextWorld for WorldGlobal {
	type ContextStack<T> = ContextStackGlobal<T>;
	type ContextStackOpaque = ContextStackGlobal<dyn StdAny>;
}

/// Handle type for the world's context stack
pub type Handle<T: ?Sized, W: ContextWorld> = <W::ContextStack<T> as ContextStack<T, W>>::Handle;

/// Opaque handle type for the world's context stack
pub type OpaqueHandle<W: ContextWorld> = <W::ContextStackOpaque as ContextStackOpaque<W>>::OpaqueHandle;

/// Bounds type for the world's context stack
pub type Bounds<T: ?Sized, W: ContextWorld> = <W::ContextStack<T> as ContextStack<T, W>>::Bounds;

/// Any type for the world's opaque context stack
pub type Any<W: ContextWorld> = <W::ContextStackOpaque as ContextStackOpaque<W>>::Any;
