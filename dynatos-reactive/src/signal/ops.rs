//! Signal operators

// Modules
mod borrow;
mod borrow_mut;
mod get;
mod get_cloned;
mod replace;
mod set;
mod update;
mod with;

// Exports
pub use self::{
	borrow::SignalBorrow,
	borrow_mut::SignalBorrowMut,
	get::{SignalGet, SignalGetCopy},
	get_cloned::{SignalGetClone, SignalGetCloned},
	replace::SignalReplace,
	set::{SignalSet, SignalSetDefaultImpl},
	update::SignalUpdate,
	with::{SignalWith, SignalWithDefaultImpl},
};
