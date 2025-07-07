//! # Memo'd signals

// Imports
use {
	crate::{
		effect::EffectSuppressed,
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

/// Memo signal.
///
/// See the module documentation for more information.
pub struct Memo<T, F: ?Sized> {
	/// Effect
	effect: Effect<EffectFn<T, F>>,
}

impl<T, F> Memo<T, F> {
	/// Creates a new memo'd signal
	#[track_caller]
	pub fn new(f: F) -> Self
	where
		T: PartialEq + 'static,
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

// TODO: `F: ?Sized`
impl<T, F> Memo<T, F> {
	/// Suppresses the update of the memo'd value
	#[track_caller]
	pub fn suppress(&self) -> EffectSuppressed<'_, impl EffectRun>
	where
		T: PartialEq + 'static,
		F: Fn() -> T + 'static,
	{
		self.effect.suppress()
	}

	/// Updates the existing value without updating dependencies
	// TODO: Just implement `SignalBorrowMut` and friends?
	pub fn update_raw(&self, value: T) {
		*self.effect.inner_fn().value.borrow_mut() = Some(value);
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

impl<T: 'static, F: ?Sized> SignalBorrow for Memo<T, F> {
	type Ref<'a>
		= BorrowRef<'a, T, F>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.effect.inner_fn().trigger.gather_subs();

		self.borrow_raw()
	}

	fn borrow_raw(&self) -> Self::Ref<'_> {
		let effect_fn = self.effect.inner_fn();
		let value = effect_fn.value.borrow();
		BorrowRef(value, PhantomData)
	}
}

impl<T: 'static, F: ?Sized> SignalWithDefaultImpl for Memo<T, F> {}
impl<T: 'static, F: ?Sized> SignalGetDefaultImpl for Memo<T, F> {}
impl<T: 'static, F: ?Sized> SignalGetClonedDefaultImpl for Memo<T, F> {}

impl<T, F: ?Sized> Clone for Memo<T, F> {
	fn clone(&self) -> Self {
		Self {
			effect: self.effect.clone(),
		}
	}
}

#[coverage(off)]
impl<T: fmt::Debug, F: ?Sized> fmt::Debug for Memo<T, F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let effect_fn = self.effect.inner_fn();
		let mut debug = f.debug_struct("Memo");
		match effect_fn.value.try_borrow() {
			Ok(value) => debug.field("value", &*value).finish(),
			Err(_) => debug.finish_non_exhaustive(),
		}
	}
}

impl<T, F1, F2> CoerceUnsized<Memo<T, F2>> for Memo<T, F1>
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
	T: PartialEq + 'static,
	F: Fn() -> T + 'static,
{
	crate::effect_run_impl_inner! {}

	fn run(&self, _ctx: EffectRunCtx<'_>) {
		let new_value = (self.f)();
		let mut value = self.value.borrow_mut();

		// Write the new value, if it's different from the previous
		// Note: Since we're comparing against `Some(_)`, any `None` values
		//       will always be written to.
		let is_same = value.as_ref() == Some(&new_value);
		if !is_same {
			*value = Some(new_value);
			drop(value);
			self.trigger.exec();
		}
	}
}
