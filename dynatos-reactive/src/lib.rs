//! Reactivity for `dynatos`

// Features
#![feature(
	unsize,
	coerce_unsized,
	unboxed_closures,
	fn_traits,
	test,
	associated_type_bounds,
	thread_local,
	lint_reasons,
	cfg_match
)]

// Modules
pub mod async_signal;
pub mod derived;
pub mod effect;
pub mod memo;
pub mod signal;
pub mod trigger;
pub mod with_default;

// Exports
pub use self::{
	async_signal::AsyncSignal,
	derived::Derived,
	effect::{Effect, WeakEffect},
	memo::Memo,
	signal::{
		Signal,
		SignalBorrow,
		SignalBorrowMut,
		SignalGet,
		SignalGetClone,
		SignalGetCloned,
		SignalGetCopy,
		SignalReplace,
		SignalSet,
		SignalSetWith,
		SignalUpdate,
		SignalWith,
	},
	trigger::{IntoSubscriber, Subscriber, Trigger, WeakTrigger},
	with_default::{SignalWithDefault, WithDefault},
};
