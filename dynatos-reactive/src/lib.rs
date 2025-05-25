//! Reactivity for `dynatos`

// TODO: Currently both effects and triggers need to keep a map
//       of dependencies/subscribers to each other, can we change
//       this to be more efficient?

// Features
#![feature(
	unsize,
	coerce_unsized,
	unboxed_closures,
	fn_traits,
	test,
	thread_local,
	cfg_match,
	trait_alias,
	once_cell_try,
	async_fn_traits,
	local_waker,
	debug_closure_helpers,
	decl_macro,
	auto_traits,
	negative_impls,
	stmt_expr_attributes,
	proc_macro_hygiene
)]

// Modules
pub mod async_signal;
pub mod derived;
pub mod effect;
pub mod memo;
pub mod signal;
pub mod trigger;
pub mod with_default;
pub mod world;

// Exports
pub use self::{
	async_signal::AsyncSignal,
	derived::Derived,
	effect::{Effect, EffectRun, WeakEffect},
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
		SignalSetDefaultImpl,
		SignalSetWith,
		SignalUpdate,
		SignalUpdateDefaultImpl,
		SignalWith,
		SignalWithDefaultImpl,
	},
	trigger::{IntoSubscriber, Subscriber, Trigger, WeakTrigger},
	with_default::{SignalWithDefault, WithDefault},
	world::ReactiveWorld,
};
