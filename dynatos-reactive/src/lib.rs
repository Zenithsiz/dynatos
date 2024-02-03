//! Reactivity for [`dynatos`]

// Modules
pub mod effect;
pub mod signal;

// Exports
pub use self::{
	effect::{Effect, WeakEffect},
	signal::Signal,
};
