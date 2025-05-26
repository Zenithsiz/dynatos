//! World

// Lints
#![expect(
	type_alias_bounds,
	reason = "Although they're not enforced currently, they will be in the future and we want to be explicit already"
)]

// TODO: Get rid of all of the `*World` types strewn about. They only exist because we can't provide
//       the necessary bounds, such as `T: Unsize<U> => Rc<T>: CoerceUnsized<Rc<U>>`.

// Modules
pub mod effect_stack;
pub mod run_queue;

// Exports
pub use self::{
	effect_stack::{EffectStack, EffectStackGlobal, EffectStackThreadLocal},
	run_queue::{RunQueue, RunQueueGlobal, RunQueueThreadLocal},
};

// Imports
use {
	crate::{effect, trigger, Effect, EffectRun, WeakTrigger},
	core::{marker::Unsize, ops::CoerceUnsized},
	dynatos_world::{IMut, Weak, World, WorldGlobal, WorldThreadLocal},
	std::collections::{HashMap, HashSet},
};

/// Reactive world
pub trait ReactiveWorldInner: World {
	/// Effect function
	type F: ?Sized + Unsize<Self::F> + 'static;

	/// Effect stack
	type EffectStack: EffectStack<Self>;

	/// Run queue
	type RunQueue: RunQueue<Self>;

	/// Unsizes an effect `Effect<F, W>` to `Effect<Self::F, W>`
	// TODO: Encode the capability somehow...
	fn unsize_effect<F>(effect: Effect<F, Self>) -> Effect<Self::F, Self>
	where
		F: ?Sized + Unsize<Self::F>,
		Self: ReactiveWorld;
}

// TODO: Remove this once we can assume these bounds, or somehow encode them into `ReactiveWorldInner`
#[expect(private_bounds, reason = "We can't *not* leak some implementation details currently")]
#[cfg_attr(
	not(debug_assertions),
	expect(
		clippy::zero_sized_map_values,
		reason = "It isn't zero sized with `debug_assertions`"
	)
)]
pub trait ReactiveWorld = ReactiveWorldInner
where
	<Self as ReactiveWorldInner>::F: EffectRun<Self>,
	Weak<effect::Inner<<Self as ReactiveWorldInner>::F, Self>, Self>:
		CoerceUnsized<Weak<effect::Inner<<Self as ReactiveWorldInner>::F, Self>, Self>>,
	IMut<HashMap<crate::Subscriber<Self>, trigger::SubscriberInfo>, Self>: Sized,
	IMut<HashSet<WeakTrigger<Self>>, Self>: Sized,
	IMut<(), Self>: Sized;

impl ReactiveWorldInner for WorldThreadLocal {
	type EffectStack = EffectStackThreadLocal;
	type F = dyn EffectRun<Self> + 'static;
	type RunQueue = RunQueueThreadLocal;

	fn unsize_effect<F>(effect: Effect<F, Self>) -> Effect<Self::F, Self>
	where
		F: ?Sized + Unsize<Self::F>,
		Self: ReactiveWorld,
	{
		effect
	}
}
impl ReactiveWorldInner for WorldGlobal {
	type EffectStack = EffectStackGlobal;
	type F = dyn EffectRun<Self> + Send + Sync + 'static;
	type RunQueue = RunQueueGlobal;

	fn unsize_effect<F>(effect: Effect<F, Self>) -> Effect<Self::F, Self>
	where
		F: ?Sized + Unsize<Self::F>,
		Self: ReactiveWorld,
	{
		effect
	}
}
