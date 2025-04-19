//! Effect stack

// Imports
use {
	crate::{effect::EffectWorld, ReactiveWorld, WeakEffect},
	core::marker::Unsize,
	dynatos_world::{IMut, IMutLike, WorldGlobal, WorldThreadLocal},
};

/// Effect stack
pub trait EffectStack<W: ReactiveWorld>: Sized {
	/// Effect function
	type F: ?Sized + Fn() + Unsize<Self::F> + 'static;

	/// Pushes an effect to the stack.
	fn push_effect<F>(f: WeakEffect<F, W>)
	where
		F: ?Sized + Unsize<Self::F>,
		W: EffectWorld;

	/// Pops an effect from the stack
	fn pop_effect();

	/// Returns the top effect of the stack
	fn top_effect() -> Option<WeakEffect<Self::F, W>>
	where
		W: EffectWorld;
}

/// Effect stack impl
type EffectStackImpl<F: ?Sized, W> = IMut<Vec<WeakEffect<F, W>>, W>;

/// Thread-local effect stack, using `StdRc` and `StdRefCell`
pub struct EffectStackThreadLocal;

/// Effect stack for `EffectStackThreadLocal`
#[thread_local]
static EFFECT_STACK_STD_RC: EffectStackImpl<dyn Fn(), WorldThreadLocal> =
	EffectStackImpl::<_, WorldThreadLocal>::new(vec![]);

impl EffectStack<WorldThreadLocal> for EffectStackThreadLocal {
	type F = dyn Fn() + 'static;

	fn push_effect<F>(f: WeakEffect<F, WorldThreadLocal>)
	where
		F: ?Sized + Unsize<Self::F>,
	{
		EFFECT_STACK_STD_RC.write().push(f);
	}

	fn pop_effect() {
		EFFECT_STACK_STD_RC.write().pop().expect("Missing added effect");
	}

	fn top_effect() -> Option<WeakEffect<Self::F, WorldThreadLocal>> {
		EFFECT_STACK_STD_RC.read().last().cloned()
	}
}

/// Global effect stack, using `StdArc` and `StdRefCell`
pub struct EffectStackGlobal;

/// Effect stack for `EffectStackGlobal`
static EFFECT_STACK_STD_ARC: EffectStackImpl<dyn Fn() + Send + Sync, WorldGlobal> =
	EffectStackImpl::<_, WorldGlobal>::new(vec![]);


impl EffectStack<WorldGlobal> for EffectStackGlobal {
	type F = dyn Fn() + Send + Sync + 'static;

	fn push_effect<F>(f: WeakEffect<F, WorldGlobal>)
	where
		F: ?Sized + Unsize<Self::F>,
	{
		EFFECT_STACK_STD_ARC.write().push(f);
	}

	fn pop_effect() {
		EFFECT_STACK_STD_ARC.write().pop().expect("Missing added effect");
	}

	fn top_effect() -> Option<WeakEffect<Self::F, WorldGlobal>> {
		EFFECT_STACK_STD_ARC.read().last().cloned()
	}
}
