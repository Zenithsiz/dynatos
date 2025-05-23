//! Effect stack

// Imports
use {
	super::{ReactiveWorld, ReactiveWorldInner},
	crate::{EffectRun, WeakEffect},
	core::marker::Unsize,
	dynatos_world::{IMut, IMutLike, WorldGlobal, WorldThreadLocal},
};

/// Effect stack
// TODO: Require `W: ReactiveWorld` once that doesn't result in a cycle overflow.
pub trait EffectStack<W>: Sized {
	/// Pushes an effect to the stack.
	fn push<F>(f: WeakEffect<F, W>)
	where
		F: ?Sized + Unsize<W::F>,
		W: ReactiveWorld;

	/// Pops an effect from the stack
	fn pop();

	/// Returns the top effect of the stack
	fn top() -> Option<WeakEffect<W::F, W>>
	where
		W: ReactiveWorld;
}

/// Effect stack impl
type EffectStackImpl<F: ?Sized, W> = IMut<Vec<WeakEffect<F, W>>, W>;

/// Thread-local effect stack, using `StdRc` and `StdRefCell`
pub struct EffectStackThreadLocal;

/// Effect stack for `EffectStackThreadLocal`
#[thread_local]
static EFFECT_STACK_STD_RC: EffectStackImpl<dyn EffectRun, WorldThreadLocal> =
	EffectStackImpl::<_, WorldThreadLocal>::new(vec![]);

impl EffectStack<WorldThreadLocal> for EffectStackThreadLocal {
	fn push<F>(f: WeakEffect<F, WorldThreadLocal>)
	where
		F: ?Sized + Unsize<<WorldThreadLocal as ReactiveWorldInner>::F>,
	{
		EFFECT_STACK_STD_RC.write().push(f);
	}

	fn pop() {
		EFFECT_STACK_STD_RC.write().pop().expect("Missing added effect");
	}

	fn top() -> Option<WeakEffect<<WorldThreadLocal as ReactiveWorldInner>::F, WorldThreadLocal>> {
		EFFECT_STACK_STD_RC.read().last().cloned()
	}
}

/// Global effect stack, using `StdArc` and `StdRefCell`
pub struct EffectStackGlobal;

/// Effect stack for `EffectStackGlobal`
static EFFECT_STACK_STD_ARC: EffectStackImpl<dyn EffectRun + Send + Sync, WorldGlobal> =
	EffectStackImpl::<_, WorldGlobal>::new(vec![]);


impl EffectStack<WorldGlobal> for EffectStackGlobal {
	fn push<F>(f: WeakEffect<F, WorldGlobal>)
	where
		F: ?Sized + Unsize<<WorldGlobal as ReactiveWorldInner>::F>,
	{
		EFFECT_STACK_STD_ARC.write().push(f);
	}

	fn pop() {
		EFFECT_STACK_STD_ARC.write().pop().expect("Missing added effect");
	}

	fn top() -> Option<WeakEffect<<WorldGlobal as ReactiveWorldInner>::F, WorldGlobal>> {
		EFFECT_STACK_STD_ARC.read().last().cloned()
	}
}
