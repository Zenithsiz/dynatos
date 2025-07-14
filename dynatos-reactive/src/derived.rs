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
//! ```rust
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
		effect,
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
	std::rc::Rc,
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
		F: DerivedRun<T> + 'static,
	{
		let value = RefCell::new(None);
		let effect = Effect::new(EffectFn {
			trigger: Trigger::new(),
			value,
			f,
		});

		Self { effect }
	}

	/// Creates a new lazy derived signal.
	///
	/// Only calls `f` when first accessing the value.
	#[track_caller]
	pub fn new_lazy(f: F) -> Self
	where
		T: 'static,
		F: DerivedRun<T> + 'static,
	{
		let value = RefCell::new(None);
		let effect = Effect::new_raw(EffectFn {
			trigger: Trigger::new(),
			value,
			f,
		});

		Self { effect }
	}
}

impl<T, F: ?Sized> Derived<T, F> {
	/// Unsizes this value into a `Derived<dyn DerivedRun<T>>`.
	// Note: This is necessary for unsizing from `!Sized` to `dyn DerivedRun`,
	//       since those coercions only work for `Sized` types.
	// TODO: Once we can unsize from `?Sized` to `dyn DerivedRun`,
	//       remove this.
	#[must_use]
	pub fn unsize(self) -> Derived<T, dyn DerivedRun<T>>
	where
		F: DerivedRun<T>,
	{
		Derived {
			effect: Effect {
				inner: self.effect.inner.unsize_inner_derived(),
			},
		}
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

#[coverage(off)]
impl<T: fmt::Debug, F: ?Sized> fmt::Debug for BorrowRef<'_, T, F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(*self.0).fmt(f)
	}
}

impl<T: 'static, F: ?Sized + DerivedRun<T> + 'static> SignalBorrow for Derived<T, F> {
	type Ref<'a>
		= BorrowRef<'a, T, F>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.effect.inner_fn().trigger.gather_subs();

		let effect_fn = self.effect.inner_fn();
		let mut value = effect_fn.value.borrow();

		// Initialize the value if we haven't
		if value.is_none() {
			drop(value);
			self.effect.run();
			value = effect_fn.value.borrow();
		}

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

#[coverage(off)]
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
#[doc(hidden)]
pub struct EffectFn<T, F: ?Sized> {
	/// Trigger
	trigger: Trigger,

	/// Value
	value: RefCell<Option<T>>,

	/// Function
	f: F,
}

// Note: This is necessary to use `EffectFn` as a receiver
//       for unsizing in `DerivedRun`.
impl<T, F: ?Sized> Deref for EffectFn<T, F> {
	type Target = F;

	fn deref(&self) -> &Self::Target {
		&self.f
	}
}

impl<T, F> EffectRun for EffectFn<T, F>
where
	T: 'static,
	F: ?Sized + DerivedRun<T> + 'static,
{
	fn run(&self, _ctx: EffectRunCtx<'_>) {
		*self.value.borrow_mut() = Some(self.f.run());
		self.trigger.exec();
	}

	fn unsize_inner(self: Rc<effect::Inner<Self>>) -> Rc<effect::Inner<dyn EffectRun>> {
		DerivedRun::unsize_inner_effect(self)
	}
}

/// Derived run
///
/// # Implementation
/// To implement this trait, you must implement the [`run`](DerivedRun::run) function,
/// and then use the macro [`derived_run_impl_inner`] to implement some details.
pub trait DerivedRun<T> {
	/// Runs the derived function, yielding a value
	fn run(&self) -> T;


	// Implementation details.


	/// Unsizes the inner field of the effect to a `dyn EffectRun`
	#[doc(hidden)]
	fn unsize_inner_effect(self: Rc<effect::Inner<EffectFn<T, Self>>>) -> Rc<effect::Inner<dyn EffectRun>>;

	/// Unsizes the inner field of the effect to an effect fn to a `dyn DerivedRun`.
	#[doc(hidden)]
	fn unsize_inner_derived(
		self: Rc<effect::Inner<EffectFn<T, Self>>>,
	) -> Rc<effect::Inner<EffectFn<T, dyn DerivedRun<T>>>>;
}

/// Implementation detail for the [`EffectRun`] trait
pub macro derived_run_impl_inner($T:ty) {
	fn unsize_inner_effect(self: Rc<effect::Inner<EffectFn<$T, Self>>>) -> Rc<effect::Inner<dyn EffectRun>> {
		self
	}

	fn unsize_inner_derived(
		self: Rc<effect::Inner<EffectFn<$T, Self>>>,
	) -> Rc<effect::Inner<EffectFn<$T, dyn DerivedRun<$T>>>> {
		self
	}
}

impl<T, F> DerivedRun<T> for F
where
	T: 'static,
	F: Fn() -> T + 'static,
{
	derived_run_impl_inner! { T }

	fn run(&self) -> T {
		self()
	}
}

#[cfg(test)]
mod tests {
	use {super::*, core::cell::Cell};

	#[test]
	fn unsize() {
		let f1 = Derived::new(|| 1_usize);
		let f2: Derived<usize, dyn DerivedRun<usize>> = f1.clone();

		assert_eq!(&f1.effect, &f2.effect);
		assert_eq!(*f2.borrow(), 1);
	}

	#[test]
	fn lazy() {
		#[thread_local]
		static COUNT: Cell<usize> = Cell::new(0);

		let f = Derived::new_lazy(|| COUNT.set(COUNT.get() + 1));
		assert_eq!(COUNT.get(), 0, "Lazy effect was run before access");
		_ = f.borrow();
		assert_eq!(COUNT.get(), 1, "Lazy effect was not run after access");
		_ = f.borrow();
		assert_eq!(COUNT.get(), 1, "Lazy effect was run again after access");
	}
}
