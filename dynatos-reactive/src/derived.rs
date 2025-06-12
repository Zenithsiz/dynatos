//! # Derived signals
//!
//! A derived signal, [`Derived`], is a signal that caches a reactive function's result, that is,
//! a function that depends on other signals.
//!
//! This is useful for splitting up an effect that requires computing multiple expensive operations,
//! to avoid needless re-computing certain values when others change.
//!
//! ## Examples
//! Without using a derived, whenever any dependent signals of `expensive_operation1` or
//! `expensive_operation2` are updated, then they will both be re-run due to `my_value`
//! requiring an update.
//! ```rust,no_run
//! // Pretend these are expensive operations
//! let expensive_operation1 = move || 1;
//! let expensive_operation2 = move || 2;
//! let my_value = move || expensive_operation1() + expensive_operation2();
//! ```
//!
//! Meanwhile, when using [`Derived`], you can cache each value, so that any updates
//! to one of the signals doesn't re-compute the other.
// TODO: Not ignore the test once we find out why it hangs the compiler
//! ```rust,no_run
//! use dynatos_reactive::{Derived, SignalGet};
//! let expensive_operation1 = Derived::new(move || 1);
//! let expensive_operation2 = Derived::new(move || 2);
//! let my_value = move || expensive_operation1.get() + expensive_operation2.get();
//! ```
//!
//! It's important to note that this isn't free however, as [`Derived`] needs to
//! not only store the latest value, it also needs to create an effect that is re-run
//! each time any dependencies are updated.

// Imports
use {
	crate::{
		Effect,
		EffectRun,
		EffectRunCtx,
		SignalBorrow,
		SignalGetClonedDefaultImpl,
		SignalGetDefaultImpl,
		SignalWithDefaultImpl,
		Trigger,
	},
	core::{
		cell::{self, RefCell},
		fmt,
		marker::{PhantomData, Unsize},
		ops::{CoerceUnsized, Deref},
	},
};

/// Derived signal.
///
/// See the module documentation for more information.
pub struct Derived<T, F: ?Sized> {
	/// Effect
	effect: Effect<EffectFn<T, F>>,
}

impl<T, F> Derived<T, F> {
	/// Creates a new derived signal
	#[track_caller]
	pub fn new(f: F) -> Self
	where
		T: 'static,
		F: Fn() -> T + 'static,
	{
		let value = RefCell::new(None);
		let effect = Effect::new(EffectFn {
			trigger: Trigger::new(),
			value,
			f,
		});

		Self { effect }
	}
}

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, T: 'a, F: ?Sized>(cell::Ref<'a, Option<T>>, PhantomData<fn(F)>);

impl<T, F: ?Sized> Deref for BorrowRef<'_, T, F> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Value wasn't initialized")
	}
}

impl<T: fmt::Debug, F: ?Sized> fmt::Debug for BorrowRef<'_, T, F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(*self.0).fmt(f)
	}
}

impl<T: 'static, F: ?Sized> SignalBorrow for Derived<T, F> {
	type Ref<'a>
		= BorrowRef<'a, T, F>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.effect.inner_fn().trigger.gather_subscribers();

		self.borrow_raw()
	}

	fn borrow_raw(&self) -> Self::Ref<'_> {
		let effect_fn = self.effect.inner_fn();
		let value = effect_fn.value.borrow();
		BorrowRef(value, PhantomData)
	}
}

impl<T: 'static, F: ?Sized> SignalWithDefaultImpl for Derived<T, F> {}
impl<T: 'static, F: ?Sized> SignalGetDefaultImpl for Derived<T, F> {}
impl<T: 'static, F: ?Sized> SignalGetClonedDefaultImpl for Derived<T, F> {}

impl<T, F: ?Sized> Clone for Derived<T, F> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
		}
	}
}

impl<T: fmt::Debug, F: ?Sized> fmt::Debug for Derived<T, F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let effect_fn = self.effect.inner_fn();
		let mut debug = f.debug_struct("Derived");
		debug.field("effect", &self.effect);
		debug.field("trigger", &effect_fn.trigger);

		match effect_fn.value.try_borrow() {
			Ok(value) => debug.field("value", &*value).finish(),
			Err(_) => debug.finish_non_exhaustive(),
		}
	}
}

impl<T, F1, F2> CoerceUnsized<Derived<T, F2>> for Derived<T, F1>
where
	F1: ?Sized + Unsize<F2>,
	F2: ?Sized,
{
}

/// Effect function
struct EffectFn<T, F: ?Sized> {
	/// Trigger
	trigger: Trigger,

	/// Value
	value: RefCell<Option<T>>,

	/// Function
	f: F,
}

impl<T, F> EffectRun for EffectFn<T, F>
where
	T: 'static,
	F: Fn() -> T,
{
	fn run(&self, _ctx: EffectRunCtx<'_>) {
		*self.value.borrow_mut() = Some((self.f)());
		self.trigger.exec();
	}
}
