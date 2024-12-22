//! Async signal

// TODO: Support wakers that wake from a separate thread
//       by using some runtime and a channel.

// TODO: Trigger whenever we finish loading the future, not just when
//       the waker wakes.

// Imports
#[cfg(not(feature = "sync"))]
use std::thread::{self, ThreadId};
use {
	crate::{SignalBorrow, SignalWith, Trigger},
	core::{
		fmt,
		future::{self, Future},
		pin::Pin,
		sync::atomic::{self, AtomicBool},
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
struct Inner<F: Future> {
	/// Future
	///
	/// SAFETY:
	/// Must not be moved out until it's finished.
	fut: IMut<Option<F>>,

	/// Waker
	waker: Arc<Waker>,

	/// Whether we're suspended
	is_suspended: AtomicBool,

	/// Value
	value: OnceLock<F::Output>,
}

/// Async signal.
///
/// # Waker
/// Currently this signal panics if the waker passed
/// into the future is woken from a different thread.
pub struct AsyncSignal<F: Future> {
	/// Inner
	inner: Pin<Rc<Inner<F>>>,
}

impl<F: Future> AsyncSignal<F> {
	/// Creates a new async signal from a future
	#[track_caller]
	pub fn new(fut: F) -> Self {
		Self::new_inner(fut, false)
	}

	/// Creates a new suspended async signal from a future
	#[track_caller]
	pub fn new_suspended(fut: F) -> Self {
		Self::new_inner(fut, true)
	}

	/// Creates a new async signal from a future
	#[track_caller]
	fn new_inner(fut: F, is_suspended: bool) -> Self {
		let inner = Rc::pin(Inner {
			fut:          IMut::new(Some(fut)),
			waker:        Arc::new(Waker {
				#[cfg(not(feature = "sync"))]
				thread: thread::current().id(),
				trigger: Trigger::new(),
			}),
			is_suspended: AtomicBool::new(is_suspended),
			value:        OnceLock::new(),
		});
		Self { inner }
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
	pub async fn load(&self) -> &'_ F::Output {
		// Gather subcribers before polling
		// TODO: Is this correct? We should probably be gathering by task,
		//       instead of by thread.
		self.inner.waker.trigger.gather_subscribers();

		// Poll until we're loaded
		future::poll_fn(|cx| match self.try_load(cx) {
			Some(value) => Poll::Ready(value),
			None => Poll::Pending,
		})
		.await
	}

	/// Inner function to try to load the future
	fn try_load(&self, cx: &mut task::Context<'_>) -> Option<&F::Output> {
		self.inner
			.value
			.get_or_try_init(|| {
				// Get the inner future through pin projection.
				let mut inner_fut = self.inner.fut.imut_write();
				let fut = inner_fut
					.as_mut()
					.expect("Future was missing without value being initialized");

				// SAFETY: We guarantee that the future is not moved until it's finished.
				let mut fut = unsafe { Pin::new_unchecked(&mut *fut) };

				// Then poll it
				match fut.as_mut().poll(cx) {
					Poll::Ready(value) => {
						// Drop the future once we load it
						let _: Option<F> = inner_fut.take();

						Ok(value)
					},
					Poll::Pending => Err(()),
				}
			})
			.ok()
	}

	/// Borrows the inner value, without polling the future.
	#[must_use]
	#[track_caller]
	pub fn borrow_suspended(&self) -> Option<&'_ F::Output> {
		self.inner.waker.trigger.gather_subscribers();
		self.inner.value.get()
	}

	/// Uses the inner value, without polling the future.
	#[track_caller]
	pub fn with_suspended<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Option<&'a F::Output>) -> O,
	{
		let borrow = self.borrow_suspended();
		f(borrow)
	}
}

impl<F: Future> Clone for AsyncSignal<F> {
	fn clone(&self) -> Self {
		Self {
			inner: Pin::clone(&self.inner),
		}
	}
}

impl<F: Future> fmt::Debug for AsyncSignal<F>
where
	F::Output: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.inner.value.get();
		f.debug_struct("AsyncSignal").field("value", &value).finish()
	}
}

impl<F: Future> SignalBorrow for AsyncSignal<F> {
	type Ref<'a>
		= Option<&'a F::Output>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		self.inner.waker.trigger.gather_subscribers();

		// If we're suspended, don't poll
		if self.is_suspended() {
			return self.inner.value.get();
		}

		// Otherwise, try to load it
		self.inner.waker.trigger.gather_subscribers();
		let waker = task::Waker::from(Arc::clone(&self.inner.waker));
		let mut cx = task::Context::from_waker(&waker);
		self.try_load(&mut cx)
	}
}

impl<F: Future> SignalWith for AsyncSignal<F>
where
	F::Output: 'static,
{
	type Value<'a> = Option<&'a F::Output>;

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
		f(value)
	}
}
