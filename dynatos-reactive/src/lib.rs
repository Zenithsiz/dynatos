//! Reactivity for `dynatos`

// Features
#![feature(
	unsize,
	coerce_unsized,
	unboxed_closures,
	fn_traits,
	test,
	thread_local,
	trait_alias,
	once_cell_try,
	async_fn_traits,
	local_waker,
	debug_closure_helpers,
	decl_macro,
	auto_traits,
	negative_impls,
	stmt_expr_attributes,
	proc_macro_hygiene,
	type_alias_impl_trait,
	macro_metavar_expr,
	try_trait_v2,
	try_trait_v2_residual,
	never_type,
	unwrap_infallible,
	arbitrary_self_types,
	coverage_attribute,
	macro_metavar_expr_concat
)]

// Modules
pub mod async_signal;
pub mod dep_graph;
pub mod derived;
pub mod effect;
pub mod effect_stack;
pub mod enum_split;
mod loc;
pub mod mapped_signal;
pub mod memo;
pub mod run_queue;
pub mod signal;
pub mod trigger;
pub mod with_default;
pub mod world;

// Exports
pub use self::{
	async_signal::AsyncSignal,
	derived::Derived,
	effect::{Effect, EffectRun, EffectRunCtx, WeakEffect, effect_run_impl_inner},
	enum_split::{EnumSplitSignal, SignalEnumSplit},
	mapped_signal::{MappedSignal, SignalMapped, TryMappedSignal},
	memo::Memo,
	signal::{
		Signal,
		SignalBorrow,
		SignalBorrowMut,
		SignalGet,
		SignalGetClone,
		SignalGetCloned,
		SignalGetClonedDefaultImpl,
		SignalGetCopy,
		SignalGetDefaultImpl,
		SignalReplace,
		SignalSet,
		SignalSetDefaultImpl,
		SignalSetWith,
		SignalUpdate,
		SignalUpdateDefaultImpl,
		SignalWith,
		SignalWithDefaultImpl,
	},
	trigger::{Trigger, WeakTrigger},
	with_default::{SignalWithDefault, WithDefault},
	world::{GLOBAL_WORLD, THREAD_WORLD, GlobalWorld},
};
