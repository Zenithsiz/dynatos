//! Signal operators

// Modules
mod borrow;
mod get;
mod get_cloned;
mod replace;
mod set;
mod update;
mod with;

// Exports
pub use self::{
	borrow::SignalBorrow,
	get::{SignalGet, SignalGetCopy},
	get_cloned::{SignalGetClone, SignalGetCloned},
	replace::SignalReplace,
	set::{SignalSet, SignalSetWith},
	update::SignalUpdate,
	with::SignalWith,
};
