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
	lint_reasons
)]

// Modules
pub mod async_signal;
pub mod derived;
pub mod effect;
pub mod signal;
pub mod trigger;
pub mod with_default;

// Exports
pub use self::{
	async_signal::AsyncSignal,
	derived::Derived,
	effect::{Effect, WeakEffect},
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
	trigger::{Trigger, WeakTrigger},
	with_default::{SignalWithDefault, WithDefault},
};

// Imports
use core::marker::Unsize;

/// Types that may be converted into a subscriber
pub trait IntoSubscriber {
	/// Converts this type into a weak effect.
	fn into_subscriber(self) -> WeakEffect<dyn Fn()>;
}

#[duplicate::duplicate_item(
	T body;
	[ Effect ] [ self.downgrade() ];
	[ &'_ Effect ] [ self.downgrade() ];
	[ WeakEffect ] [ self ];
)]
impl<F> IntoSubscriber for T<F>
where
	F: ?Sized + Fn() + Unsize<dyn Fn()> + 'static,
{
	fn into_subscriber(self) -> WeakEffect<dyn Fn()> {
		body
	}
}
