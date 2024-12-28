//! Async signal

// TODO: Support wakers that wake from a separate thread
//       by using some runtime and a channel.

// Imports
use {
	crate::{SignalBorrow, SignalWith, Trigger},
	core::{
		fmt,
		future::{self, Future},
		marker::PhantomPinned,
		mem,
		ops::Deref,
		pin::Pin,
		task::{self, Poll},
	},
	dynatos_reactive_sync::{IMut, IMutExt, IMutRef, IMutRefMut, IMutRefMutExt, Rc},
};

/// Inner
struct Inner<F: AsyncFnMut<()> + 'static> {
	/// Loader
	loader: IMut<InnerLoader<F>>,

	/// Trigger
	trigger: Trigger,

	/// Value
	value: IMut<Option<F::Output>>,
}

/// Async signal.
///
/// # Waker
/// Currently this signal panics if the waker passed
/// into the future is woken from a different thread.
pub struct AsyncSignal<F: AsyncFnMut<()> + 'static> {
	/// Inner
	inner: Pin<Rc<Inner<F>>>,
}

impl<F: AsyncFnMut<()> + 'static> AsyncSignal<F> {
	/// Creates a new async signal with a loader
	#[track_caller]
	#[must_use]
	pub fn new(loader: F) -> Self {
		Self::new_inner(loader, false)
	}

	/// Creates a new async signal with a loader and starts loading it
	#[track_caller]
	#[must_use]
	pub fn new_loading(loader: F) -> Self {
		Self::new_inner(loader, true)
	}

	/// Inner constructor
	#[track_caller]
	fn new_inner(loader: F, loading: bool) -> Self {
		let fut = match loading {
			true => InnerLoader::new_loading(loader),
			false => InnerLoader::new(loader),
		};

		let inner = Rc::pin(Inner {
			loader:  IMut::new(fut),
			trigger: Trigger::new(),
			value:   IMut::new(None),
		});
		Self { inner }
	}

	/// Stops loading the value.
	///
	/// Returns if the loader had a future.
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn stop_loading(&self) -> bool {
		self.inner.loader.imut_write().clear_fut()
	}

	/// Starts loading the value.
	///
	/// If the loader already has a future, this does nothing.
	///
	/// Returns whether this created the loader's future.
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn start_loading(&self) -> bool
	where
		F: AsyncFnMut<()>,
	{
		// Reset the future if it's `None`
		let mut loader = self.inner.loader.imut_write();
		if loader.fut().is_none() {
			loader.reset_fut();
		}


		// If we have a load waker, wake it
		let load_waker = loader.take_waker();
		if let Some(load_waker) = load_waker {
			// Note: Before waking, drop the lock, or we might deadlock
			drop(loader);
			load_waker.wake();
		}

		true
	}

	/// Restarts the loading.
	///
	/// If the loader already has a future, it will be dropped
	/// and re-created.
	///
	/// Returns whether a future existed before
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn restart_loading(&self) -> bool
	where
		F: AsyncFnMut<()>,
	{
		// Reset the future
		let mut loader = self.inner.loader.imut_write();
		let had_fut = loader.reset_fut();

		// If we have a load waker, wake it
		let load_waker = loader.take_waker();
		if let Some(load_waker) = load_waker {
			// Note: Before waking, drop the lock, or we might deadlock
			drop(loader);
			load_waker.wake();
		}

		had_fut
	}

	/// Returns if loading.
	///
	/// This is considered loading if the loader has a future active.
	#[must_use]
	pub fn is_loading(&self) -> bool {
		self.inner.loader.imut_read().fut().is_some()
	}

	/// Waits for the value to be loaded.
	///
	/// If not loading, waits until the loading starts, but does not start it.
	pub async fn wait(&self) -> BorrowRef<'_, F::Output> {
		// Poll until we're loaded
		future::poll_fn(|cx| match self.try_load(cx) {
			Some(value) => Poll::Ready(value),
			None => Poll::Pending,
		})
		.await
	}

	/// Loads the inner value.
	///
	/// If already loaded, returns it without loading.
	///
	/// Otherwise, this will start loading.
	///
	/// If this future is dropped before completion, the loading
	/// will be cancelled.
	pub async fn load(&self) -> BorrowRef<'_, F::Output>
	where
		F: AsyncFnMut<()>,
	{
		{
			let value = self.inner.value.imut_read();
			if value.is_some() {
				return BorrowRef(value);
			}
		}

		// Create the loader, and the guard that drops it if we created it.
		// Note: The clear is a no-op if `wait` successfully returns, we only
		//       care if we're dropped early.
		let created_loader = self.start_loading();
		scopeguard::defer! {
			if created_loader {
				self.stop_loading();
			}
		}

		self.wait().await
	}

	/// Inner function to try to load the future
	fn try_load<'a>(&'a self, cx: &mut task::Context<'_>) -> Option<BorrowRef<'a, F::Output>> {
		// If the value is loaded, return it
		let mut value = self.inner.value.imut_write();
		if value.is_some() {
			return Some(BorrowRef(IMutRefMut::imut_downgrade(value)));
		}

		// Store this waker so we can react to it whenever the future changes.
		// Note: This must be done under the lock of
		let mut loader = self.inner.loader.imut_write();
		loader.set_waker(cx.waker().clone());


		// Get the inner future through pin projection.
		// Note: If there is none, we return unsuccessfully, but since we
		//       saved the waker, we'll eventually be awaken when a future
		//       is set again.
		let mut inner_fut = loader.fut_mut();
		let mut fut = inner_fut.as_mut().as_pin_mut()?;

		// Then poll it
		// TODO: Is it safe to call `poll` with the `'static` lifetime?
		//       You can't specialize on lifetimes, and we're not handing out
		//       the value to user code.
		let value = match fut.as_mut().poll(cx) {
			Poll::Ready(new_value) => {
				// Drop the future once we load it
				// Note: Assignment drops the previous value in-place, so this
				//       is fine even if it was pinned.
				Pin::set(&mut inner_fut, None);

				// Assign the new value
				*value = Some(new_value);

				// Then downgrade the lock
				let value = IMutRefMut::imut_downgrade(value);

				// And trigger all dependencies before returning
				self.inner.trigger.trigger();

				value
			},
			Poll::Pending => return None,
		};

		Some(BorrowRef(value))
	}

	/// Borrows the inner value, without polling the loader's future.
	#[must_use]
	#[track_caller]
	pub fn borrow_suspended(&self) -> Option<BorrowRef<'_, F::Output>> {
		self.inner.trigger.gather_subscribers();
		let value = self.inner.value.imut_read();
		value.is_some().then(|| BorrowRef(value))
	}

	/// Uses the inner value, without polling the loader's future.
	#[track_caller]
	pub fn with_suspended<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Option<BorrowRef<'a, F::Output>>) -> O,
	{
		let borrow = self.borrow_suspended();
		f(borrow)
	}
}

impl<F: AsyncFnMut<()> + 'static> Clone for AsyncSignal<F> {
	fn clone(&self) -> Self {
		Self {
			inner: Pin::clone(&self.inner),
		}
	}
}

impl<F: AsyncFnMut<()> + 'static> fmt::Debug for AsyncSignal<F>
where
	F::Output: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.borrow_suspended();
		f.debug_struct("AsyncSignal").field("value", &value).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T>(IMutRef<'a, Option<T>>);

impl<T> Deref for BorrowRef<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Borrow was `None`")
	}
}

impl<F: AsyncFnMut<()> + 'static> SignalBorrow for AsyncSignal<F> {
	type Ref<'a>
		= Option<BorrowRef<'a, F::Output>>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		// Try to load it
		self.inner.trigger.gather_subscribers();

		#[cfg(feature = "sync")]
		let waker = {
			let raw_waker = self.inner.trigger.clone().into_raw_waker();
			// SAFETY: `Trigger` ensures we can create a `Waker` from it, if
			//         `sync` is active.
			unsafe { task::Waker::from_raw(raw_waker) }
		};

		// TODO: Here we could still use the trigger as a local waker.
		#[cfg(not(feature = "sync"))]
		let waker = {
			let waker = no_sync_waker::NoSyncWaker::new(self.inner.trigger.clone());
			let waker = std::sync::Arc::new(waker);
			task::Waker::from(waker)
		};

		let mut cx = task::Context::from_waker(&waker);
		self.try_load(&mut cx)
	}
}

impl<F: AsyncFnMut<()> + 'static> SignalWith for AsyncSignal<F>
where
	F::Output: 'static,
{
	type Value<'a> = Option<BorrowRef<'a, F::Output>>;

	#[track_caller]
	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(value)
	}
}

/// Inner loader.
///
/// This is a separate module to ensure we can't access the fields of [`InnerLoader`]
/// directly, as that would be easy to cause U.B. with.
use inner_loader::*;
mod inner_loader {
	use super::*;

	/// Inner loader
	pub struct InnerLoader<F: AsyncFnMut<()> + 'static> {
		/// Loading future
		///
		/// SAFETY:
		/// - Must not be moved out until it's finished.
		/// - Must be cast to `F::CallRefFuture<'loader>` when accessing.
		/// - Must be dropped *before* the loader
		fut: Option<F::CallRefFuture<'static>>,

		/// Loader
		loader: F,

		/// Waker
		///
		/// Used for waking external runtimes from [`AsyncSignal::wait`] and [`AsyncSignal::load`].
		///
		/// In particular, this is only used when replacing the future, since for
		/// progressing the future, the waker already gets passed into it.
		// TODO: Should we only be storing the latest one? We can have multiple waiters/
		//       loaders (that are waiting), and they might be using different wakers.
		waker: Option<task::Waker>,

		/// Phantom marker
		_pinned: PhantomPinned,
	}

	impl<F: AsyncFnMut<()> + 'static> InnerLoader<F> {
		/// Creates a new loader
		pub const fn new(loader: F) -> Self {
			Self {
				fut: None,
				loader,
				waker: None,
				_pinned: PhantomPinned,
			}
		}

		/// Creates a new loader, and starts the future
		pub fn new_loading(mut loader: F) -> Self {
			let fut = loader();
			// SAFETY: We ensure the future is dropped before the loader
			let fut = unsafe { mem::transmute::<F::CallRefFuture<'_>, F::CallRefFuture<'static>>(fut) };

			Self {
				fut: Some(fut),
				loader,
				waker: None,
				_pinned: PhantomPinned,
			}
		}

		/// Clears this loader's future.
		///
		/// Returns if the future existed.
		pub fn clear_fut(&mut self) -> bool {
			let mut fut = self.fut_mut();
			let has_fut = fut.is_some();
			Pin::set(&mut fut, None);

			has_fut
		}

		/// Resets this loader's future.
		///
		/// Drops the existing future, if it exists
		///
		/// Returns whether a future existed before
		pub fn reset_fut(&mut self) -> bool {
			// Drop the existing fut, if any
			// Note: This must be done because calling `self.loader` will invalidate it.
			let had_fut = self.clear_fut();

			let fut = (self.loader)();
			// SAFETY: We ensure the future is dropped before the loader
			let fut = unsafe { mem::transmute::<F::CallRefFuture<'_>, F::CallRefFuture<'static>>(fut) };
			self.fut = Some(fut);

			had_fut
		}

		/// Returns this loader's future
		pub const fn fut(&self) -> &Option<F::CallRefFuture<'static>> {
			&self.fut
		}

		/// Returns this loader's future mutably pinned.
		pub fn fut_mut(&mut self) -> Pin<&mut Option<F::CallRefFuture<'static>>> {
			// SAFETY: We do not hand out `&mut` references to this field, so this is
			//         just projection after locking.
			unsafe { Pin::new_unchecked(&mut self.fut) }
		}

		/// Takes this loader's waker
		pub const fn take_waker(&mut self) -> Option<task::Waker> {
			self.waker.take()
		}

		/// Sets the waker
		pub fn set_waker(&mut self, waker: task::Waker) {
			self.waker = Some(waker);
		}
	}
}

/// Waker for when `sync` is disabled.
#[cfg(not(feature = "sync"))]
mod no_sync_waker {
	use {
		self::mem::ManuallyDrop,
		super::*,
		std::{
			sync::Arc,
			task::Wake,
			thread::{self, ThreadId},
		},
	};

	/// Waker.
	pub struct NoSyncWaker {
		/// Active thread
		thread: ThreadId,

		/// Trigger
		trigger: Trigger,
	}

	impl NoSyncWaker {
		/// Creates a new waker
		pub fn new(trigger: Trigger) -> Self {
			Self {
				thread: thread::current().id(),
				trigger,
			}
		}
	}

	impl Wake for NoSyncWaker {
		fn wake(self: Arc<Self>) {
			// Note: We need to wrap the rc in a manually drop, else
			//       by dropping we'll be updating the reference count
			//       from possibly another thread (if `wake_by_ref` panics).
			//       This does leak memory if this panics, but that should be
			//       acceptable behavior, since this isn't intended behavior.
			let this = ManuallyDrop::new(self);
			this.wake_by_ref();

			ManuallyDrop::into_inner(this);
		}

		fn wake_by_ref(self: &Arc<Self>) {
			// Ensure we're on the same thread as we were created
			// TODO: Could accessing `self.thread` be UB?
			assert_eq!(
				thread::current().id(),
				self.thread,
				"`AsyncSignal` may only be used with futures that wake in the same thread. You may enable the `sync` \
				 feature to allow this behavior."
			);

			// Then trigger
			self.trigger.trigger();
		}
	}

	// SAFETY: We ensure that the inner trigger is only accessed
	//         in the same thread as it was created on.
	#[expect(clippy::non_send_fields_in_send_ty, reason = "See SAFETY")]
	const _: () = {
		unsafe impl Send for NoSyncWaker {}
		unsafe impl Sync for NoSyncWaker {}
	};
}
