//! Loadable signal

// Imports
use {
	crate::Loadable,
	core::{fmt, future::Future, ops::Deref},
	dynatos_reactive::{AsyncSignal, SignalBorrow, SignalWith},
};

/// Loadable signal.
///
/// Wrapper around an [`AsyncSignal`].
pub struct LoadableSignal<F: Future> {
	/// Inner
	inner: AsyncSignal<F>,
}

impl<F: Future<Output = Result<T, E>>, T, E> LoadableSignal<F> {
	/// Creates a new loadable signal from a future
	#[track_caller]
	pub fn new(fut: F) -> Self {
		Self {
			inner: AsyncSignal::new(fut),
		}
	}

	/// Creates a new suspended loadable signal from a future
	///
	/// Using this signal will not advance the inner future.
	#[track_caller]
	pub fn new_suspended(fut: F) -> Self {
		Self {
			inner: AsyncSignal::new_suspended(fut),
		}
	}

	/// Sets whether this future should be suspended
	pub fn set_suspended(&self, is_suspended: bool) {
		self.inner.set_suspended(is_suspended);
	}

	/// Gets whether this future should is suspended
	#[must_use]
	pub fn is_suspended(&self) -> bool {
		self.inner.is_suspended()
	}

	/// Gets whether this future has been polled
	#[must_use]
	pub fn has_polled(&self) -> bool {
		self.inner.has_polled()
	}

	/// Loads this value asynchronously and returns the value
	pub async fn load(&self) -> Result<BorrowRef<'_, T, E>, E>
	where
		F: Future<Output = Result<T, E>> + 'static,
		T: 'static,
		E: Clone + 'static,
	{
		let value = self.inner.load().await;
		match value {
			Ok(_) => Ok(BorrowRef(value)),
			Err(err) => Err(err.clone()),
		}
	}

	/// Borrows the inner value, without polling the future.
	#[must_use]
	pub fn borrow_suspended(&self) -> Loadable<BorrowRef<'_, T, E>, E>
	where
		E: Clone,
	{
		let borrow = self.inner.borrow_suspended();
		match borrow {
			Some(borrow) => match borrow {
				Ok(_) => Loadable::Loaded(BorrowRef(borrow)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}

	/// Uses the inner value, without polling the future.
	pub fn with_suspended<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Loadable<&'a T, E>) -> O,
		E: Clone,
	{
		let borrow = self.borrow_suspended();
		f(borrow.as_deref())
	}
}

impl<F: Future> Clone for LoadableSignal<F> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
		}
	}
}

impl<F: Future<Output = Result<T, E>>, T, E> fmt::Debug for LoadableSignal<F>
where
	T: fmt::Debug,
	E: Clone + fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let loadable = self.borrow_suspended();
		f.debug_struct("LoadableSignal").field("loadable", &loadable).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T, E>(&'a Result<T, E>);

impl<T, E> Deref for BorrowRef<'_, T, E> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
			.as_ref()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<F: Future<Output = Result<T, E>> + 'static, T: 'static, E: Clone + 'static> SignalBorrow for LoadableSignal<F> {
	type Ref<'a>
		= Loadable<BorrowRef<'a, T, E>, E>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		let borrow = self.inner.borrow();
		match borrow {
			Some(borrow) => match borrow {
				Ok(_) => Loadable::Loaded(BorrowRef(borrow)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}
}

impl<F: Future<Output = Result<T, E>> + 'static, T: 'static, E: Clone + 'static> SignalWith for LoadableSignal<F> {
	type Value<'a> = Loadable<&'a T, E>;

	#[track_caller]
	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(value.as_deref())
	}
}
