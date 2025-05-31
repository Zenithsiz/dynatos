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
	get::{SignalGet, SignalGetCopy, SignalGetDefaultImpl},
	get_cloned::{SignalGetClone, SignalGetCloned, SignalGetClonedDefaultImpl},
	replace::SignalReplace,
	set::{SignalSet, SignalSetDefaultImpl, SignalSetWith},
	update::{SignalUpdate, SignalUpdateDefaultImpl},
	with::{SignalWith, SignalWithDefaultImpl},
};
