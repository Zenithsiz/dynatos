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

// Exports
pub use self::effect_stack::{EffectStack, EffectStackGlobal, EffectStackThreadLocal};

// Imports
use {
	crate::{effect, trigger, WeakEffect, WeakTrigger},
	core::{marker::Unsize, ops::CoerceUnsized},
	dynatos_world::{IMut, Weak, World, WorldGlobal, WorldThreadLocal},
	std::collections::{HashMap, HashSet},
};

/// Reactive world
pub trait ReactiveWorldInner: World {
	/// Effect stack
	type EffectStack: EffectStack<Self>;
}

// TODO: Remove this once we can assume these bounds, or somehow encode them into `ReactiveWorldInner`
#[expect(private_bounds, reason = "We can't *not* leak some implementation details currently")]
pub trait ReactiveWorld = ReactiveWorldInner
where
	Weak<effect::Inner<F<Self>, Self>, Self>: CoerceUnsized<Weak<effect::Inner<F<Self>, Self>, Self>>,
	IMut<HashMap<crate::Subscriber<Self>, trigger::SubscriberInfo>, Self>: Sized,
	IMut<HashSet<WeakTrigger<Self>>, Self>: Sized;

impl ReactiveWorldInner for WorldThreadLocal {
	type EffectStack = EffectStackThreadLocal;
}
impl ReactiveWorldInner for WorldGlobal {
	type EffectStack = EffectStackGlobal;
}

/// The effect stack function type of the world `W`
pub type F<W: ReactiveWorld> = <W::EffectStack as EffectStack<W>>::F;

/// `Unsize` into the effect stack function of the world `W`
pub trait UnsizeF<W: ReactiveWorld> = Unsize<F<W>>;

/// Pushes an effect onto the effect stack of the world `W`
pub fn push_effect<F, W>(effect: WeakEffect<F, W>)
where
	F: ?Sized + Unsize<self::F<W>>,
	W: ReactiveWorld,
{
	W::EffectStack::push_effect(effect);
}

/// Pops an effect onto the effect stack of the world `W`
pub fn pop_effect<W>()
where
	W: ReactiveWorld,
{
	W::EffectStack::pop_effect();
}

/// Returns the top of the effect stack of the world `W`
#[must_use]
pub fn top_effect<W>() -> Option<WeakEffect<F<W>, W>>
where
	W: ReactiveWorld,
{
	W::EffectStack::top_effect()
}
