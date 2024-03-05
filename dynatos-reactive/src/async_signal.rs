//! Async signal

// TODO: Support wakers that wake from a separate thread
//       by using some runtime and a channel.

// Imports
use {
	crate::{effect, SignalWith, Trigger},
	pin_cell::PinCell,
	std::{
		cell::OnceCell,
		fmt,
		future::Future,
		pin::Pin,
		rc::Rc,
		sync::Arc,
		task::{self, Poll},
		thread::{self, ThreadId},
	},
};

/// Waker
struct Waker {
	/// Active thread
	thread: ThreadId,

	/// Trigger
	trigger: Trigger,
}

impl task::Wake for Waker {
	fn wake(self: Arc<Self>) {
		self.wake_by_ref();
	}

	fn wake_by_ref(self: &Arc<Self>) {
		// If we're not in the same thread, panic
		if thread::current().id() != self.thread {
			panic!("`AsyncSignal` may only be used with futures that wake in the same thread.");
		}

		self.trigger.trigger();
	}
}

// SAFETY: We ensure that the inner trigger is only accessed
//         in the same thread as it was created on.
unsafe impl Send for Waker {}
unsafe impl Sync for Waker {}

/// Inner
#[pin_project::pin_project]
struct Inner<F: Future> {
	/// Future
	#[pin]
	fut: PinCell<F>,

	/// Waker
	waker: Arc<Waker>,

	/// Value
	value: OnceCell<F::Output>,
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
	pub fn new(fut: F) -> Self {
		let inner = Rc::pin(Inner {
			fut:   PinCell::new(fut),
			waker: Arc::new(Waker {
				thread:  thread::current().id(),
				trigger: Trigger::new(),
			}),
			value: OnceCell::new(),
		});
		Self { inner }
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

impl<F: Future> SignalWith for AsyncSignal<F>
where
	F::Output: 'static,
{
	type Value<'a> = Option<&'a F::Output>;

	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		// If there's an effect running, add it to the subscribers
		if let Some(effect) = effect::running() {
			self.inner.waker.trigger.add_subscriber(effect);
		}

		// Then try to poll the future, if we don't have a value yet
		if self.inner.value.get().is_none() {
			// Get the inner future through pin-project + pin-cell.
			let inner = self.inner.as_ref().project_ref();
			let mut fut = inner.fut.borrow_mut();
			let fut = pin_cell::PinMut::as_mut(&mut fut);

			// Then poll it, and store the value if finished.
			let waker = task::Waker::from(Arc::clone(&self.inner.waker));
			let mut cx = task::Context::from_waker(&waker);
			if let Poll::Ready(value) = fut.poll(&mut cx) {
				self.inner
					.value
					.set(value)
					.unwrap_or_else(|_| panic!("Value was already initialized"));
			}
		}

		// Finally use the value
		f(self.inner.value.get())
	}
}
