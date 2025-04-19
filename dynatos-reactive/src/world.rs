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
	crate::{effect::EffectWorld, WeakEffect},
	core::marker::Unsize,
	dynatos_world::{World, WorldGlobal, WorldThreadLocal},
};

/// Reactive world
pub trait ReactiveWorld: World {
	/// Effect stack
	type EF: EffectStack<Self>;
}

impl ReactiveWorld for WorldThreadLocal {
	type EF = EffectStackThreadLocal;
}
impl ReactiveWorld for WorldGlobal {
	type EF = EffectStackGlobal;
}

/// The effect stack function type of the world `W`
pub type F<W: ReactiveWorld> = <W::EF as EffectStack<W>>::F;

/// `Unsize` into the effect stack function of the world `W`
pub trait UnsizeF<W: ReactiveWorld> = Unsize<F<W>>;

/// Pushes an effect onto the effect stack of the world `W`
pub fn push_effect<F, W>(effect: WeakEffect<F, W>)
where
	W: EffectWorld,
	F: ?Sized + Unsize<self::F<W>>,
{
	W::EF::push_effect(effect);
}

/// Pops an effect onto the effect stack of the world `W`
pub fn pop_effect<W>()
where
	W: EffectWorld,
{
	W::EF::pop_effect();
}

/// Returns the top of the effect stack of the world `W`
#[must_use]
pub fn top_effect<W>() -> Option<WeakEffect<F<W>, W>>
where
	W: EffectWorld,
{
	W::EF::top_effect()
}
