//! Reference-counted pointers

pub mod strong;
pub mod weak;

pub use self::{strong::RcPtr, weak::WeakRcPtr};
