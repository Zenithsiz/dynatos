//! Loadable signal

// Imports
use {
	crate::Loadable,
	core::{
		fmt,
		future::Future,
		ops::{Deref, DerefMut},
	},
	dynatos_reactive::{async_signal, AsyncSignal, SignalBorrow, SignalBorrowMut, SignalUpdate, SignalWith},
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
	pub fn new(fut: F) -> Self {
		let inner = AsyncSignal::new(fut);
		Self { inner }
	}

	/// Borrows the inner value, without polling the future.
	#[must_use]
	pub fn borrow_inner(&self) -> Loadable<BorrowRef<'_, T, E>, E>
	where
		E: Clone,
	{
		let borrow = self.inner.borrow_inner();
		match borrow {
			Some(borrow) => match &*borrow {
				Ok(_) => Loadable::Loaded(BorrowRef(borrow)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}

	/// Uses the inner value, without polling the future.
	pub fn with_inner<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Loadable<&'a T, E>) -> O,
		E: Clone,
	{
		let borrow = self.borrow_inner();
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
		let loadable = self.borrow_inner();
		f.debug_struct("LoadableSignal").field("loadable", &loadable).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T, E>(async_signal::BorrowRef<'a, Result<T, E>>);

impl<'a, T, E> Deref for BorrowRef<'a, T, E> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
			.as_ref()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<F: Future<Output = Result<T, E>> + 'static, T: 'static, E: Clone + 'static> SignalBorrow for LoadableSignal<F> {
	type Ref<'a> = Loadable<BorrowRef<'a, T, E>, E>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		let borrow = self.inner.borrow();
		match borrow {
			Some(borrow) => match &*borrow {
				Ok(_) => Loadable::Loaded(BorrowRef(borrow)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}
}

impl<F: Future<Output = Result<T, E>> + 'static, T: 'static, E: Clone + 'static> SignalWith for LoadableSignal<F> {
	type Value<'a> = Loadable<&'a T, E>;

	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(value.as_deref())
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a, T, E>(async_signal::BorrowRefMut<'a, Result<T, E>>);

impl<'a, T, E> Deref for BorrowRefMut<'a, T, E> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
			.as_ref()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<'a, T, E> DerefMut for BorrowRefMut<'a, T, E> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.0
			.as_mut()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<F: Future<Output = Result<T, E>>, T: 'static, E: Clone + 'static> SignalBorrowMut for LoadableSignal<F> {
	type RefMut<'a> = Loadable<BorrowRefMut<'a, T, E>, E>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let borrow = self.inner.borrow_mut();
		match borrow {
			Some(borrow) => match &*borrow {
				Ok(_) => Loadable::Loaded(BorrowRefMut(borrow)),
				Err(err) => Loadable::Err(err.clone()),
			},
			None => Loadable::Empty,
		}
	}
}

/// Updates the value within the loadable signal.
impl<F: Future<Output = Result<T, E>>, T: 'static, E: Clone + 'static> SignalUpdate for LoadableSignal<F>
where
	F::Output: 'static,
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
