//! Loadable signal

// Imports
use {
	crate::Loadable,
	core::{
		fmt,
		ops::{Deref, DerefMut},
	},
	dynatos_reactive::{
		async_signal::{self, Loader},
		AsyncSignal,
		SignalBorrow,
		SignalBorrowMut,
		SignalGetClone,
		SignalGetClonedDefaultImpl,
		SignalGetCopy,
		SignalGetDefaultImpl,
		SignalSetDefaultImpl,
		SignalUpdate,
		SignalUpdateDefaultImpl,
		SignalWith,
		SignalWithDefaultImpl,
	},
};

/// Loadable signal.
///
/// Wrapper around an [`AsyncSignal`].
pub struct LoadableSignal<F: Loader> {
	/// Inner
	inner: AsyncSignal<F>,
}

impl<F> LoadableSignal<F>
where
	F: Loader,
{
	/// Creates a new async signal with a loader
	#[track_caller]
	#[must_use]
	pub fn new(loader: F) -> Self {
		Self {
			inner: AsyncSignal::new(loader),
		}
	}

	/// Stops the loading future.
	///
	/// See [`AsyncSignal::stop_loading`] for details
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn stop_loading(&self) -> bool {
		self.inner.stop_loading()
	}

	/// Starts a new loading future.
	///
	/// See [`AsyncSignal::start_loading`] for details
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn start_loading(&self) -> bool {
		self.inner.start_loading()
	}

	/// Restarts the currently loading future.
	///
	/// See [`AsyncSignal::restart_loading`] for details
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn restart_loading(&self) -> bool
where {
		self.inner.restart_loading()
	}

	/// Returns if there exists a loading future.
	#[must_use]
	pub fn is_loading(&self) -> bool {
		self.inner.is_loading()
	}
}

impl<F, T, E> LoadableSignal<F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static,
	E: Clone + 'static,
{
	/// Borrows the value, without loading it
	#[must_use]
	#[track_caller]
	pub fn borrow_unloaded(&self) -> Loadable<BorrowRef<'_, F>, E> {
		let res = self.inner.borrow_unloaded();
		match res {
			Some(res) => match &*res {
				Ok(_) => Loadable::Loaded(BorrowRef(res)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}

	/// Borrows the value, without loading it or gathering subscribers
	#[must_use]
	#[track_caller]
	pub fn borrow_unloaded_raw(&self) -> Loadable<BorrowRef<'_, F>, E> {
		let res = self.inner.borrow_unloaded_raw();
		match res {
			Some(res) => match &*res {
				Ok(_) => Loadable::Loaded(BorrowRef(res)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}
}

impl<F> Clone for LoadableSignal<F>
where
	F: Loader,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
		}
	}
}

impl<F, T, E> fmt::Debug for LoadableSignal<F>
where
	F: Loader<Output = Result<T, E>>,
	T: fmt::Debug + 'static,
	E: Clone + fmt::Debug + 'static,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("LoadableSignal").field(&self.inner).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, F: Loader>(async_signal::BorrowRef<'a, F>);

impl<F, T, E> fmt::Debug for BorrowRef<'_, F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static + fmt::Debug,
	E: 'static,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(**self).fmt(f)
	}
}

impl<F, T, E> Deref for BorrowRef<'_, F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static,
	E: 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
			.as_ref()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<F, T, E> SignalGetCopy for Loadable<BorrowRef<'_, F>, E>
where
	F: Loader<Output = Result<T, E>>,
	T: Copy + 'static,
	E: 'static,
{
	type Value = Loadable<T, E>;

	fn copy_value(self) -> Self::Value {
		self.map(|value| *value)
	}
}

impl<F, T, E> SignalGetClone for Loadable<BorrowRef<'_, F>, E>
where
	F: Loader<Output = Result<T, E>>,
	T: Clone + 'static,
	E: 'static,
{
	type Value = Loadable<T, E>;

	fn clone_value(self) -> Self::Value {
		self.map(|value| value.clone())
	}
}

impl<F, T, E> SignalBorrow for LoadableSignal<F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static,
	E: Clone + 'static,
{
	type Ref<'a>
		= Loadable<BorrowRef<'a, F>, E>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		let res = self.inner.borrow();
		match res {
			Some(res) => match &*res {
				Ok(_) => Loadable::Loaded(BorrowRef(res)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}
}

impl<F, T, E> SignalWith for LoadableSignal<F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static,
	E: Clone + 'static,
{
	type Value<'a> = Loadable<&'a T, E>;

	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(value.as_deref())
	}
}

/// Mutable reference type for [`SignalBorrow`] impl
pub struct BorrowRefMut<'a, F: Loader>(async_signal::BorrowRefMut<'a, F>);

impl<F, T, E> fmt::Debug for BorrowRefMut<'_, F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static + fmt::Debug,
	E: 'static,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(**self).fmt(f)
	}
}

impl<F, T, E> Deref for BorrowRefMut<'_, F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static,
	E: 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
			.as_ref()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<F, T, E> DerefMut for BorrowRefMut<'_, F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static,
	E: 'static,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.0
			.as_mut()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<F, T, E> SignalBorrowMut for LoadableSignal<F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static,
	E: Clone + 'static,
{
	type RefMut<'a>
		= Loadable<BorrowRefMut<'a, F>, E>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let res = self.inner.borrow_mut();
		match res {
			Some(res) => match &*res {
				Ok(_) => Loadable::Loaded(BorrowRefMut(res)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}
}

impl<F, T, E> SignalUpdate for LoadableSignal<F>
where
	F: Loader<Output = Result<T, E>>,
	T: 'static,
	E: Clone + 'static,
{
	type Value<'a> = Loadable<&'a mut T, E>;

	fn update<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut value = self.borrow_mut();
		f(value.as_deref_mut())
	}
}

impl<F: Loader> SignalSetDefaultImpl for LoadableSignal<F> {}
impl<F: Loader> SignalGetDefaultImpl for LoadableSignal<F> {}
impl<F: Loader> SignalGetClonedDefaultImpl for LoadableSignal<F> {}

// Note: We want to return a `Loadable<&T>` instead of `&Loadable<T>`,
//       so we can't use the default impl
impl<F: Loader> !SignalWithDefaultImpl for LoadableSignal<F> {}
impl<F: Loader> !SignalUpdateDefaultImpl for LoadableSignal<F> {}
