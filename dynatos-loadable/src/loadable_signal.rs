//! Loadable signal

// Imports
use {
	crate::Loadable,
	core::{fmt, ops::Deref},
	dynatos_reactive::{SignalBorrow, SignalWith},
	dynatos_reactive_async::{async_signal, AsyncSignal},
};

/// Loadable signal.
///
/// Wrapper around an [`AsyncSignal`].
pub struct LoadableSignal<F: AsyncFnMut<()> + 'static> {
	/// Inner
	inner: AsyncSignal<F>,
}

impl<F> LoadableSignal<F>
where
	F: AsyncFnMut<()> + 'static,
{
	/// Creates a new async signal with a loader
	#[track_caller]
	#[must_use]
	pub fn new(loader: F) -> Self {
		Self {
			inner: AsyncSignal::new(loader),
		}
	}

	/// Creates a new async signal with a loader and starts loading it
	#[track_caller]
	#[must_use]
	pub fn new_loading(loader: F) -> Self {
		Self {
			inner: AsyncSignal::new_loading(loader),
		}
	}

	/// Stops loading the value.
	///
	/// Returns if the loader had a future.
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn stop_loading(&self) -> bool {
		self.inner.stop_loading()
	}

	/// Starts loading the value.
	///
	/// If the loader already has a future, this does nothing.
	///
	/// Returns whether this created the loader's future.
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn start_loading(&self) -> bool {
		self.inner.start_loading()
	}

	/// Restarts the loading.
	///
	/// If the loader already has a future, it will be dropped
	/// and re-created.
	///
	/// Returns whether a future existed before
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn restart_loading(&self) -> bool {
		self.inner.restart_loading()
	}

	/// Returns if loading.
	///
	/// This is considered loading if the loader has a future active.
	#[must_use]
	pub fn is_loading(&self) -> bool {
		self.inner.is_loading()
	}
}

impl<F, T, E> LoadableSignal<F>
where
	F: AsyncFnMut() -> Result<T, E> + 'static,
{
	/// Waits for the value to be loaded.
	///
	/// If not loading, waits until the loading starts, but does not start it.
	pub async fn wait(&self) -> Result<BorrowRef<'_, T, E>, E>
	where
		T: 'static,
		E: Clone + 'static,
	{
		let res = self.inner.wait().await;
		match &*res {
			Ok(_) => Ok(BorrowRef(res)),
			Err(err) => Err(err.clone()),
		}
	}

	/// Loads the inner value.
	///
	/// If already loaded, returns it without loading.
	///
	/// Otherwise, this will start loading.
	///
	/// If this future is dropped before completion, the loading
	/// will be cancelled.
	pub async fn load(&self) -> Result<BorrowRef<'_, T, E>, E>
	where
		T: 'static,
		E: Clone + 'static,
	{
		let res = self.inner.load().await;
		match &*res {
			Ok(_) => Ok(BorrowRef(res)),
			Err(err) => Err(err.clone()),
		}
	}

	/// Borrows the inner value, without polling the loader's future.
	#[must_use]
	pub fn borrow_suspended(&self) -> Loadable<BorrowRef<'_, T, E>, E>
	where
		E: Clone + 'static,
	{
		let res = self.inner.borrow_suspended();
		match res {
			Some(res) => match &*res {
				Ok(_) => Loadable::Loaded(BorrowRef(res)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}

	/// Uses the inner value, without polling the loader's future.
	pub fn with_suspended<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Loadable<BorrowRef<'a, T, E>, E>) -> O,
		E: Clone + 'static,
	{
		let res = self.borrow_suspended();
		f(res)
	}
}

impl<F> Clone for LoadableSignal<F>
where
	F: AsyncFnMut<()>,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
		}
	}
}

impl<F, T, E> fmt::Debug for LoadableSignal<F>
where
	F: AsyncFnMut() -> Result<T, E>,
	T: fmt::Debug,
	E: Clone + fmt::Debug + 'static,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let loadable = self.borrow_suspended();
		f.debug_struct("LoadableSignal").field("loadable", &loadable).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T, E>(async_signal::BorrowRef<'a, Result<T, E>>);

impl<T, E> Deref for BorrowRef<'_, T, E> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
			.as_ref()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<F, T, E> SignalBorrow for LoadableSignal<F>
where
	F: AsyncFnMut() -> Result<T, E>,
	T: 'static,
	E: Clone + 'static,
{
	type Ref<'a>
		= Loadable<BorrowRef<'a, T, E>, E>
	where
		Self: 'a;

	#[track_caller]
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
	F: AsyncFnMut() -> Result<T, E>,
	T: 'static,
	E: Clone + 'static,
{
	type Value<'a> = Loadable<BorrowRef<'a, T, E>, E>;

	#[track_caller]
	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(value)
	}
}
