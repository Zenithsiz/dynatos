//! # Memo'd signals

// Imports
use {
	crate::{
		trigger::TriggerWorld,
		world::UnsizeF,
		Effect,
		EffectRun,
		ReactiveWorld,
		SignalBorrow,
		SignalWith,
		Trigger,
	},
	core::{
		fmt,
		marker::{PhantomData, Unsize},
		ops::{CoerceUnsized, Deref},
	},
	dynatos_world::{IMut, IMutLike, IMutRef, WorldDefault},
};

/// World for [`Memo`]
pub trait MemoWorld<T, F: ?Sized> = ReactiveWorld + TriggerWorld where IMut<Option<T>, Self>: Sized;

/// Memo signal.
///
/// See the module documentation for more information.
pub struct Memo<T, F: ?Sized, W: MemoWorld<T, F> = WorldDefault> {
	/// Effect
	effect: Effect<EffectFn<T, F, W>, W>,
}

impl<T, F> Memo<T, F, WorldDefault> {
	/// Creates a new memo'd signal
	#[track_caller]
	pub fn new(f: F) -> Self
	where
		T: PartialEq + 'static,
		F: Fn() -> T + 'static,
	{
		Self::new_in(f, WorldDefault::default())
	}
}

impl<T, F, W: MemoWorld<T, F>> Memo<T, F, W> {
	/// Creates a new memo'd signal in a world
	#[track_caller]
	#[expect(private_bounds, reason = "We can't *not* leak some implementation details currently")]
	pub fn new_in(f: F, world: W) -> Self
	where
		T: PartialEq + 'static,
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

/// World for [`BorrowRef`]
pub trait BorrowRefWorld<'a, T, F: ?Sized> = MemoWorld<T, F> where IMut<Option<T>, Self>: 'a;

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, T: 'a, F: ?Sized, W: BorrowRefWorld<'a, T, F> = WorldDefault>(
	IMutRef<'a, Option<T>, W>,
	PhantomData<fn(F)>,
);

impl<'a, T, F: ?Sized, W: BorrowRefWorld<'a, T, F>> Deref for BorrowRef<'a, T, F, W> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Value wasn't initialized")
	}
}

impl<'a, T: fmt::Debug, F: ?Sized, W: BorrowRefWorld<'a, T, F>> fmt::Debug for BorrowRef<'a, T, F, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(*self.0).fmt(f)
	}
}

impl<T: 'static, F: ?Sized, W: MemoWorld<T, F>> SignalBorrow for Memo<T, F, W> {
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

impl<T: 'static, F: ?Sized, W: MemoWorld<T, F>> SignalWith for Memo<T, F, W> {
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

impl<T, F: ?Sized, W: MemoWorld<T, F>> Clone for Memo<T, F, W> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
		}
	}
}

impl<T: fmt::Debug, F: ?Sized, W: MemoWorld<T, F>> fmt::Debug for Memo<T, F, W> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let effect_fn = self.effect.inner_fn();
		let mut debug = f.debug_struct("Memo");
		match effect_fn.value.try_read() {
			Some(value) => debug.field("value", &*value).finish(),
			None => debug.finish_non_exhaustive(),
		}
	}
}

impl<T, F1, F2, W> CoerceUnsized<Memo<T, F2, W>> for Memo<T, F1, W>
where
	F1: ?Sized + Unsize<F2>,
	F2: ?Sized,
	W: MemoWorld<T, F1> + MemoWorld<T, F2>,
	Effect<EffectFn<T, F1, W>, W>: CoerceUnsized<Effect<EffectFn<T, F2, W>, W>>,
{
}

/// Effect function
struct EffectFn<T, F: ?Sized, W: MemoWorld<T, F>> {
	/// Trigger
	trigger: Trigger<W>,

	/// Value
	value: IMut<Option<T>, W>,

	/// Function
	f: F,
}

impl<T, F, W> EffectRun for EffectFn<T, F, W>
where
	T: PartialEq + 'static,
	F: Fn() -> T,
	W: MemoWorld<T, F>,
{
	#[track_caller]
	fn run(&self) {
		let new_value = (self.f)();
		let mut value = self.value.write();

		// Write the new value, if it's different from the previous
		// Note: Since we're comparing against `Some(_)`, any `None` values
		//       will always be written to.
		let is_same = value.as_ref() == Some(&new_value);
		if !is_same {
			*value = Some(new_value);
			drop(value);
			self.trigger.trigger();
		}
	}
}
