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
	crate::{world::UnsizeF, Effect, EffectRun, ReactiveWorld, SignalBorrow, SignalWith, Trigger},
	core::{
		fmt,
		marker::{PhantomData, Unsize},
		ops::{CoerceUnsized, Deref},
	},
	dynatos_world::{IMut, IMutLike, IMutRef, WorldDefault},
};

/// World for [`Derived`]
pub trait DerivedWorld<T, F: ?Sized> = ReactiveWorld where IMut<Option<T>, Self>: Sized;

/// Derived signal.
///
/// See the module documentation for more information.
pub struct Derived<T, F: ?Sized, W: DerivedWorld<T, F> = WorldDefault> {
	/// Effect
	effect: Effect<EffectFn<T, F, W>, W>,
}

impl<T, F> Derived<T, F, WorldDefault> {
	/// Creates a new derived signal
	#[track_caller]
	pub fn new(f: F) -> Self
	where
		T: 'static,
		F: Fn() -> T + 'static,
	{
		Self::new_in(f, WorldDefault::default())
	}
}

impl<T, F, W: DerivedWorld<T, F>> Derived<T, F, W> {
	/// Creates a new derived signal in a world
	#[track_caller]
	#[expect(private_bounds, reason = "We can't *not* leak some implementation details currently")]
	pub fn new_in(f: F, world: W) -> Self
	where
		T: 'static,
		F: Fn() -> T + 'static,
		EffectFn<T, F, W>: UnsizeF<W>,
	{
		let value = IMut::<_, W>::new(None);
		let effect = Effect::new_in(
			EffectFn {
				trigger: Trigger::new_in(world.clone()),
				value,
				f,
			},
			world,
		);

		Self { effect }
	}
}

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, T: 'a, F: ?Sized, W: DerivedWorld<T, F> = WorldDefault>(
	IMutRef<'a, Option<T>, W>,
	PhantomData<fn(F)>,
);

impl<'a, T, F: ?Sized, W: DerivedWorld<T, F>> Deref for BorrowRef<'a, T, F, W> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Value wasn't initialized")
	}
}

impl<'a, T: fmt::Debug, F: ?Sized, W: DerivedWorld<T, F>> fmt::Debug for BorrowRef<'a, T, F, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(*self.0).fmt(f)
	}
}

impl<T: 'static, F: ?Sized, W: DerivedWorld<T, F>> SignalBorrow for Derived<T, F, W> {
	type Ref<'a>
		= BorrowRef<'a, T, F, W>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		self.effect.inner_fn().trigger.gather_subscribers();

		let effect_fn = self.effect.inner_fn();
		let value = effect_fn.value.read();
		BorrowRef(value, PhantomData)
	}
}

impl<T: 'static, F: ?Sized, W: DerivedWorld<T, F>> SignalWith for Derived<T, F, W> {
	type Value<'a> = &'a T;

	#[track_caller]
	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(&value)
	}
}

impl<T, F: ?Sized, W: DerivedWorld<T, F>> Clone for Derived<T, F, W> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
		}
	}
}

impl<T: fmt::Debug, F: ?Sized, W: DerivedWorld<T, F>> fmt::Debug for Derived<T, F, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let effect_fn = self.effect.inner_fn();
		let mut debug = f.debug_struct("Derived");
		match effect_fn.value.try_read() {
			Some(value) => debug.field("value", &*value).finish(),
			None => debug.finish_non_exhaustive(),
		}
	}
}

impl<T, F1, F2, W> CoerceUnsized<Derived<T, F2, W>> for Derived<T, F1, W>
where
	F1: ?Sized + Unsize<F2>,
	F2: ?Sized,
	W: DerivedWorld<T, F1> + DerivedWorld<T, F2>,
	Effect<EffectFn<T, F1, W>, W>: CoerceUnsized<Effect<EffectFn<T, F2, W>, W>>,
{
}

/// Effect function
struct EffectFn<T, F: ?Sized, W: DerivedWorld<T, F>> {
	/// Trigger
	trigger: Trigger<W>,

	/// Value
	value: IMut<Option<T>, W>,

	/// Function
	f: F,
}

impl<T, F, W> EffectRun for EffectFn<T, F, W>
where
	T: 'static,
	F: Fn() -> T,
	W: DerivedWorld<T, F>,
{
	#[track_caller]
	fn run(&self) {
		*self.value.write() = Some((self.f)());
		self.trigger.trigger();
	}
}
