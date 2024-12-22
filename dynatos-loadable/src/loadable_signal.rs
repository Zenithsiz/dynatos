//! Loadable signal

// Imports
use {
	crate::Loadable,
	core::{
		fmt,
		future::Future,
		ops::{Deref, DerefMut},
		sync::atomic::{self, AtomicBool},
	},
	dynatos_reactive::{async_signal, AsyncSignal, SignalBorrow, SignalBorrowMut, SignalUpdate, SignalWith},
	dynatos_reactive_sync::Rc,
};

/// Inner
struct Inner<F: Future> {
	/// Signal
	signal: AsyncSignal<F>,

	/// Whether we're suspended
	is_suspended: AtomicBool,
}

/// Loadable signal.
///
/// Wrapper around an [`AsyncSignal`].
pub struct LoadableSignal<F: Future> {
	/// Inner
	inner: Rc<Inner<F>>,
}

impl<F: Future<Output = Result<T, E>>, T, E> LoadableSignal<F> {
	/// Creates a new loadable signal from a future
	#[track_caller]
	pub fn new(fut: F) -> Self {
		Self {
			inner: Rc::new(Inner {
				signal:       AsyncSignal::new(fut),
				is_suspended: AtomicBool::new(false),
			}),
		}
	}

	/// Creates a new suspended loadable signal from a future
	///
	/// Using this signal will not advance the inner future.
	#[track_caller]
	pub fn new_suspended(fut: F) -> Self {
		Self {
			inner: Rc::new(Inner {
				signal:       AsyncSignal::new(fut),
				is_suspended: AtomicBool::new(true),
			}),
		}
	}

	/// Sets whether this future should be suspended
	pub fn set_suspended(&self, is_suspended: bool) {
		self.inner.is_suspended.store(is_suspended, atomic::Ordering::Release);
	}

	/// Gets whether this future should is suspended
	#[must_use]
	pub fn is_suspended(&self) -> bool {
		self.inner.is_suspended.load(atomic::Ordering::Acquire)
	}

	/// Loads this value asynchronously and returns the value
	pub async fn load(&self) -> Result<BorrowRef<'_, T, E>, E>
	where
		F: Future<Output = Result<T, E>> + 'static,
		T: 'static,
		E: Clone + 'static,
	{
		let value = self.inner.signal.load().await;
		match &*value {
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
		let borrow = self.inner.signal.borrow_inner();
		match borrow {
			Some(borrow) => match &*borrow {
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
			inner: Rc::clone(&self.inner),
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
pub struct BorrowRef<'a, T, E>(async_signal::BorrowRef<'a, Result<T, E>>);

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
		// If we're suspended, borrow without polling
		if self.is_suspended() {
			return self.borrow_suspended();
		}

		let borrow = self.inner.signal.borrow();
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

	#[track_caller]
	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		// If we're suspended, use without polling
		if self.is_suspended() {
			return self.with_suspended(f);
		}

		let value = self.borrow();
		f(value.as_deref())
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a, T, E>(async_signal::BorrowRefMut<'a, Result<T, E>>);

impl<T, E> Deref for BorrowRefMut<'_, T, E> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
			.as_ref()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<T, E> DerefMut for BorrowRefMut<'_, T, E> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.0
			.as_mut()
			.unwrap_or_else(|_| panic!("Loadable should not be an error"))
	}
}

impl<F: Future<Output = Result<T, E>>, T: 'static, E: Clone + 'static> SignalBorrowMut for LoadableSignal<F> {
	type RefMut<'a>
		= Loadable<BorrowRefMut<'a, T, E>, E>
	where
		Self: 'a;

	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_> {
		// Note: No need to check if we're suspended, `borrow_mut` doesn't poll
		let borrow = self.inner.signal.borrow_mut();
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

	#[track_caller]
	fn update<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		// Note: No need to check if we're suspended, `borrow_mut` doesn't poll
		let mut value = self.borrow_mut();
		f(value.as_deref_mut())
	}
}
