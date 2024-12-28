//! Asynchronous reactivity for `dynatos`

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
	local_waker
)]

// Modules
pub mod async_signal;

// Exports
pub use self::async_signal::AsyncSignal;
