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
pub mod imut;
pub mod rc;

// Exports
pub use self::{
	effect_stack::{EffectStack, EffectStackGlobal, EffectStackThreadLocal},
	imut::{IMutFamily, IMutLike, IMutRefLike, IMutRefMutLike, ParkingLotRwLock, StdRefcell},
	rc::{RcFamily, RcLike, StdArc, StdRc, WeakLike},
};

// Imports
use {
	crate::{effect::EffectWorld, WeakEffect},
	core::marker::Unsize,
};

/// World
pub trait World: Sized + 'static {
	/// Reference-counted pointer family
	type RC: RcFamily;

	/// Inner mutability family
	type IM: IMutFamily;

	/// Effect stack
	type EF: EffectStack<Self>;
}

/// Thread-local world
pub struct WorldThreadLocal;

impl World for WorldThreadLocal {
	type EF = EffectStackThreadLocal;
	type IM = StdRefcell;
	type RC = StdRc;
}

/// Global world
pub struct WorldGlobal;

impl World for WorldGlobal {
	type EF = EffectStackGlobal;
	type IM = ParkingLotRwLock;
	type RC = StdArc;
}

/// The `Rc` of the world `W`
pub type Rc<T: ?Sized, W: World> = <W::RC as RcFamily>::Rc<T>;

/// The `Weak` of the world `W`
pub type Weak<T: ?Sized, W: World> = <W::RC as RcFamily>::Weak<T>;

/// The `IMut` of the world `W`
pub type IMut<T: ?Sized, W: World> = <W::IM as IMutFamily>::IMut<T>;

/// The `IMutRef` of the world `W`
pub type IMutRef<'a, T: ?Sized + 'a, W: World> = <IMut<T, W> as IMutLike<T>>::Ref<'a>;

/// The `IMutRefMut` of the world `W`
pub type IMutRefMut<'a, T: ?Sized + 'a, W: World> = <IMut<T, W> as IMutLike<T>>::RefMut<'a>;

/// The effect stack function type of the world `W`
pub type F<W: World> = <W::EF as EffectStack<W>>::F;

/// `Unsize` into the effect stack function of the world `W`
pub trait UnsizeF<W: World> = Unsize<F<W>>;

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

/// Default world
pub type WorldDefault = WorldThreadLocal;
