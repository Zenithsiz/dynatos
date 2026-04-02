//! Reference-counted pointers

// Modules
pub mod strong;
pub mod weak;

// Exports
pub use self::{strong::RcPtr, weak::WeakRcPtr};
