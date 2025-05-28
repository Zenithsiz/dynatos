//! Loadable borrow

// Imports
use {
	crate::Loadable,
	core::{
		fmt,
		ops::{Deref, DerefMut},
	},
	dynatos_reactive::{SignalBorrow, SignalBorrowMut},
};

/// Loadable borrow.
///
/// Allows transforming a signal borrow `Borrow<Loadable<T, E>>` into a
/// `Loadable<Borrow<T>, E>`.
#[derive(Clone, Copy)]
pub struct LoadableBorrow<B>(B);

impl<B, T, E> fmt::Debug for LoadableBorrow<B>
where
	B: Deref<Target = Loadable<T, E>>,
	T: fmt::Debug,
	E: 'static,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(**self).fmt(f)
	}
}

impl<B, T, E> Deref for LoadableBorrow<B>
where
	B: Deref<Target = Loadable<T, E>>,
	E: 'static,
{
	type Target = T;

	#[track_caller]
	fn deref(&self) -> &Self::Target {
		match &*self.0 {
			Loadable::Loaded(value) => value,
			_ => panic!("Loadable should be loaded"),
		}
	}
}

/// Extension trait to borrow with a [`LoadableBorrow`]
#[extend::ext(name = SignalBorrowLoadable)]
pub impl<S, T, E> S
where
	S: for<'a> SignalBorrow<Ref<'a>: Deref<Target = Loadable<T, E>>>,
	E: Clone,
{
	/// Borrows this signal as a `Loadable<Borrow<T>, E>`
	#[track_caller]
	fn borrow_loadable(&self) -> Loadable<LoadableBorrow<S::Ref<'_>>, E> {
		let borrow = self.borrow();
		match &*borrow {
			Loadable::Empty => Loadable::Empty,
			Loadable::Err(err) => Loadable::Err(err.clone()),
			Loadable::Loaded(_) => Loadable::Loaded(LoadableBorrow(borrow)),
		}
	}

	/// Borrows this signal as a `Loadable<Borrow<T>, E>`, without adding a dependency
	#[track_caller]
	fn borrow_loadable_raw(&self) -> Loadable<LoadableBorrow<S::Ref<'_>>, E> {
		let borrow = self.borrow_raw();
		match &*borrow {
			Loadable::Empty => Loadable::Empty,
			Loadable::Err(err) => Loadable::Err(err.clone()),
			Loadable::Loaded(_) => Loadable::Loaded(LoadableBorrow(borrow)),
		}
	}
}

/// Loadable mutable borrow.
///
/// Allows transforming a signal mutable borrow `Borrow<Loadable<T, E>>` into a
/// `Loadable<Borrow<T>, E>`.
#[derive(Clone, Copy)]
pub struct LoadableBorrowMut<B>(B);

impl<B, T, E> fmt::Debug for LoadableBorrowMut<B>
where
	B: Deref<Target = Loadable<T, E>>,
	T: fmt::Debug,
	E: 'static,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(**self).fmt(f)
	}
}

impl<B, T, E> Deref for LoadableBorrowMut<B>
where
	B: Deref<Target = Loadable<T, E>>,
	E: 'static,
{
	type Target = T;

	#[track_caller]
	fn deref(&self) -> &Self::Target {
		match &*self.0 {
			Loadable::Loaded(value) => value,
			_ => panic!("Loadable should be loaded"),
		}
	}
}

impl<B, T, E> DerefMut for LoadableBorrowMut<B>
where
	B: DerefMut<Target = Loadable<T, E>>,
	E: 'static,
{
	#[track_caller]
	fn deref_mut(&mut self) -> &mut Self::Target {
		match &mut *self.0 {
			Loadable::Loaded(value) => value,
			_ => panic!("Loadable should be loaded"),
		}
	}
}

/// Extension trait to borrow with a [`LoadableBorrowMut`]
#[extend::ext(name = SignalBorrowMutLoadable)]
pub impl<S, T, E> S
where
	S: for<'a> SignalBorrowMut<RefMut<'a>: DerefMut<Target = Loadable<T, E>>>,
	E: Clone,
{
	/// Borrows this signal as a `Loadable<Borrow<T>, E>`
	fn borrow_mut_loadable(&self) -> Loadable<LoadableBorrow<S::RefMut<'_>>, E> {
		let borrow = self.borrow_mut();
		match &*borrow {
			Loadable::Empty => Loadable::Empty,
			Loadable::Err(err) => Loadable::Err(err.clone()),
			Loadable::Loaded(_) => Loadable::Loaded(LoadableBorrow(borrow)),
		}
	}

	/// Borrows this signal as a `Loadable<Borrow<T>, E>`, without adding a dependency
	fn borrow_mut_loadable_raw(&self) -> Loadable<LoadableBorrow<S::RefMut<'_>>, E> {
		let borrow = self.borrow_mut_raw();
		match &*borrow {
			Loadable::Empty => Loadable::Empty,
			Loadable::Err(err) => Loadable::Err(err.clone()),
			Loadable::Loaded(_) => Loadable::Loaded(LoadableBorrow(borrow)),
		}
	}
}
