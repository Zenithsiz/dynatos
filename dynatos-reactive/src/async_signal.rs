//! Async signal

// TODO: Support wakers that wake from a separate thread
//       by using some runtime and a channel.

// Imports
use {
	crate::{effect, signal, SignalBorrow, SignalBorrowMut, SignalUpdate, SignalWith, Trigger},
	pin_cell::PinCell,
	std::{
		cell::{self, RefCell},
		fmt,
		future::Future,
		ops::{Deref, DerefMut},
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
	value: RefCell<Option<F::Output>>,
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
			value: RefCell::new(None),
		});
		Self { inner }
	}

	/// Borrows the inner value, without polling the future.
	// TODO: Better name that indicates that we don't poll?
	pub fn borrow_inner(&self) -> Option<BorrowRef<'_, F::Output>> {
		// If there's an effect running, add it to the subscribers
		if let Some(effect) = effect::running() {
			self.inner.waker.trigger.add_subscriber(effect);
		}

		let borrow = self.inner.value.borrow();
		borrow.is_some().then(|| BorrowRef(borrow))
	}

	/// Uses the inner value, without polling the future.
	// TODO: Better name that indicates that we don't poll?
	pub fn with_inner<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Option<&'a F::Output>) -> O,
	{
		let borrow = self.borrow_inner();
		f(borrow.as_deref())
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
		let value = self.inner.value.borrow();
		f.debug_struct("AsyncSignal").field("value", &value).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T>(cell::Ref<'a, Option<T>>);

impl<'a, T> Deref for BorrowRef<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Value wasn't initialized")
	}
}

impl<F: Future> SignalBorrow for AsyncSignal<F> {
	type Ref<'a> = Option<BorrowRef<'a, F::Output>>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		// Try to poll the future, if we don't have a value yet
		if self.inner.value.borrow().is_none() {
			// Get the inner future through pin-project + pin-cell.
			let inner = self.inner.as_ref().project_ref();
			let mut fut = inner.fut.borrow_mut();
			let fut = pin_cell::PinMut::as_mut(&mut fut);

			// Then poll it, and store the value if finished.
			let waker = task::Waker::from(Arc::clone(&self.inner.waker));
			let mut cx = task::Context::from_waker(&waker);
			if let Poll::Ready(value) = fut.poll(&mut cx) {
				*self.inner.value.borrow_mut() = Some(value);
			}
		}

		self.borrow_inner()
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
		let value = self.borrow();
		f(value.as_deref())
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a, T> {
	/// Value
	value: cell::RefMut<'a, Option<T>>,

	/// Trigger on drop
	// Note: Must be dropped *after* `value`.
	_trigger_on_drop: signal::TriggerOnDrop<'a>,
}

impl<'a, T> Deref for BorrowRefMut<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.value.as_ref().expect("Value wasn't initialized")
	}
}

impl<'a, T> DerefMut for BorrowRefMut<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.value.as_mut().expect("Value wasn't initialized")
	}
}

impl<F: Future> SignalBorrowMut for AsyncSignal<F> {
	type RefMut<'a> = Option<BorrowRefMut<'a, F::Output>>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.inner.value.borrow_mut();
		value.is_some().then(|| BorrowRefMut {
			value,
			_trigger_on_drop: signal::TriggerOnDrop(&self.inner.waker.trigger),
		})
	}
}

/// Updates the value within the async signal.
///
/// Does not poll the inner future, and does not allow
/// early initializing the signal.
impl<F: Future> SignalUpdate for AsyncSignal<F>
where
	F::Output: 'static,
{
	type Value<'a> = Option<&'a mut F::Output>;

	fn update<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut value = self.borrow_mut();
		f(value.as_deref_mut())
	}
}