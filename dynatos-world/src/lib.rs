//! `dynatos`'s world types.

// Features
#![feature(
	unsize,
	coerce_unsized,
	unboxed_closures,
	fn_traits,
	test,
	thread_local,
	cfg_match,
	trait_alias,
	once_cell_try,
	async_fn_traits,
	local_waker,
	cell_update
)]
// Lints
#![expect(
	type_alias_bounds,
	reason = "Although they're not enforced currently, they will be in the future and we want to be explicit already"
)]

// TODO: Get rid of all of the `*World` types strewn about. They only exist because we can't provide
//       the necessary bounds, such as `T: Unsize<U> => Rc<T>: CoerceUnsized<Rc<U>>`.

// Modules
pub mod imut;
pub mod rc;

// Exports
pub use self::{
	imut::{IMutFamily, IMutLike, IMutRefLike, IMutRefMutLike, ParkingLotRwLock, StdRefcell},
	rc::{RcFamily, RcLike, StdArc, StdRc, WeakLike},
};

/// World
pub trait World: Sized + Clone + 'static {
	/// Reference-counted pointer family
	type RC: RcFamily;

	/// Inner mutability family
	type IM: IMutFamily;
}

/// Thread-local world
#[derive(Clone, Copy, Default)]
pub struct WorldThreadLocal;

impl World for WorldThreadLocal {
	type IM = StdRefcell;
	type RC = StdRc;
}

/// Global world
#[derive(Clone, Copy, Default)]
pub struct WorldGlobal;

impl World for WorldGlobal {
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

/// Default world
pub type WorldDefault = WorldThreadLocal;
