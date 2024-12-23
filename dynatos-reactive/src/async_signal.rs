//! Async signal

// TODO: Support wakers that wake from a separate thread
//       by using some runtime and a channel.

// Imports
#[cfg(not(feature = "sync"))]
use std::thread::{self, ThreadId};
use {
	crate::{SignalBorrow, SignalWith, Trigger},
	core::{
		fmt,
		future::{self, Future},
		marker::PhantomPinned,
		mem,
		pin::Pin,
		task::{self, Poll},
	},
	dynatos_reactive_sync::{IMut, IMutExt, Rc},
	std::{
		sync::{Arc, OnceLock},
		task::Wake,
	},
};

/// Waker
struct Waker {
	/// Active thread
	#[cfg(not(feature = "sync"))]
	thread: ThreadId,

	/// Trigger
	trigger: Trigger,
}

impl Wake for Waker {
	fn wake(self: Arc<Self>) {
		self.wake_by_ref();
	}

	fn wake_by_ref(self: &Arc<Self>) {
		// Ensure we're on the same thread as we were created, if `sync`
		// isn't enabled.
		#[cfg(not(feature = "sync"))]
		assert_eq!(
			thread::current().id(),
			self.thread,
			"`AsyncSignal` may only be used with futures that wake in the same thread."
		);

		self.trigger.trigger();
	}
}

// SAFETY: We ensure that the inner trigger is only accessed
//         in the same thread as it was created on.
#[expect(clippy::non_send_fields_in_send_ty, reason = "See SAFETY")]
#[cfg(not(feature = "sync"))]
const _: () = {
	unsafe impl Send for Waker {}
	unsafe impl Sync for Waker {}
};

/// Inner
struct Inner<F: AsyncFnMut<()> + 'static> {
	/// Loader
	loader: IMut<InnerLoader<F>>,

	/// Waker
	waker: Arc<Waker>,

	/// Load waker
	///
	/// Used by [`AsyncSignal::try_load`] for notifications
	/// on the future itself changing.
	///
	/// This is a separate field from `waker`, since it might
	/// be an external waker, since [`AsyncSignal::load`] is
	/// an `async fn`, which accepts any task context.
	// TODO: This is a bit of a weird solution, should we just not store
	//       `waker` and only store this?
	load_waker: IMut<Option<task::Waker>>,

	/// Value
	value: OnceLock<F::Output>,
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
			loader:     IMut::new(fut),
			waker:      Arc::new(Waker {
				#[cfg(not(feature = "sync"))]
				thread: thread::current().id(),
				trigger: Trigger::new(),
			}),
			load_waker: IMut::new(None),
			value:      OnceLock::new(),
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
		{
			let mut fut = self.inner.loader.imut_write();
			if fut.fut().is_none() {
				fut.reset_fut();
			}
		}

		// If we have a load waker, wake it
		let load_waker = self.inner.load_waker.imut_write().take();
		if let Some(load_waker) = load_waker {
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
		let had_fut = self.inner.loader.imut_write().reset_fut();

		// If we have a load waker, wake it
		let load_waker = self.inner.load_waker.imut_write().take();
		if let Some(load_waker) = load_waker {
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
	pub async fn wait(&self) -> &'_ F::Output {
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
	pub async fn load(&self) -> &'_ F::Output
	where
		F: AsyncFnMut<()>,
	{
		if let Some(value) = self.inner.value.get() {
			return value;
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
	fn try_load<'a>(&'a self, cx: &mut task::Context<'_>) -> Option<&'a F::Output> {
		// Try to load the value
		let mut initialized = false;
		let value = self
			.inner
			.value
			.get_or_try_init(|| {
				// Store this waker so we can react to it whenever the future changes.
				*self.inner.load_waker.imut_write() = Some(cx.waker().clone());

				// Get the inner future through pin projection.
				// Note: If there is none, we return unsuccessfully, but since we
				//       saved the waker, we'll eventually be awaken when a future
				//       is set again.
				let mut inner_fut = self.inner.loader.imut_write();
				let mut inner_fut = inner_fut.fut_mut();
				let Some(mut fut) = inner_fut.as_mut().as_pin_mut() else {
					return Err(());
				};

				// Then poll it
				// TODO: Is it safe to call `poll` with the `'static` lifetime?
				//       You can't specialize on lifetimes, and we're not handing out
				//       the value to user code.
				match fut.as_mut().poll(cx) {
					Poll::Ready(value) => {
						// Drop the future once we load it
						// Note: Assignment drops the previous value in-place, so this
						//       is fine even if it was pinned.
						Pin::set(&mut inner_fut, None);

						initialized = true;
						Ok(value)
					},
					Poll::Pending => Err(()),
				}
			})
			.ok()?;

		// If we initialized it, trigger our trigger to ensure subscribers get woken
		if initialized {
			self.inner.waker.trigger.trigger();
		}

		Some(value)
	}

	/// Borrows the inner value, without polling the loader's future.
	#[must_use]
	#[track_caller]
	pub fn borrow_suspended(&self) -> Option<&'_ F::Output> {
		self.inner.waker.trigger.gather_subscribers();
		self.inner.value.get()
	}

	/// Uses the inner value, without polling the loader's future.
	#[track_caller]
	pub fn with_suspended<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Option<&'a F::Output>) -> O,
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

impl<F: AsyncFnMut<()> + 'static> SignalBorrow for AsyncSignal<F> {
	type Ref<'a>
		= Option<&'a F::Output>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		// Try to load it
		self.inner.waker.trigger.gather_subscribers();
		let waker = task::Waker::from(Arc::clone(&self.inner.waker));
		let mut cx = task::Context::from_waker(&waker);
		self.try_load(&mut cx)
	}
}

impl<F: AsyncFnMut<()> + 'static> SignalWith for AsyncSignal<F>
where
	F::Output: 'static,
{
	type Value<'a> = Option<&'a F::Output>;

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

		/// Phantom marker
		_pinned: PhantomPinned,
	}

	impl<F: AsyncFnMut<()> + 'static> InnerLoader<F> {
		/// Creates a new loader
		pub const fn new(loader: F) -> Self {
			Self {
				fut: None,
				loader,
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
	}
}
