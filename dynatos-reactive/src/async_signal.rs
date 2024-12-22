//! Async signal

// TODO: Support wakers that wake from a separate thread
//       by using some runtime and a channel.

// Imports
#[cfg(not(feature = "sync"))]
use std::thread::{self, ThreadId};
use {
	crate::{signal, SignalBorrow, SignalBorrowMut, SignalUpdate, SignalWith, Trigger},
	core::{
		fmt,
		future::{self, Future},
		ops::{Deref, DerefMut},
		pin::Pin,
		sync::atomic::{self, AtomicBool},
		task::{self, Poll},
	},
	dynatos_reactive_sync::{IMut, IMutExt, IMutRef, IMutRefMut, Rc},
	std::{sync::Arc, task::Wake},
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
	/// Must not be moved out until we're dropped.
	fut: IMut<F>,

	/// Waker
	waker: Arc<Waker>,

	/// Whether we're suspended
	is_suspended: AtomicBool,

	/// Value
	value: IMut<Option<F::Output>>,
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
			fut:          IMut::new(fut),
			waker:        Arc::new(Waker {
				#[cfg(not(feature = "sync"))]
				thread: thread::current().id(),
				trigger: Trigger::new(),
			}),
			is_suspended: AtomicBool::new(is_suspended),
			value:        IMut::new(None),
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
	pub async fn load(&self) -> BorrowRef<'_, F::Output> {
		// Poll until we're loaded
		future::poll_fn(|cx| {
			// Get the inner future through pin projection.
			let mut fut = self.inner.fut.imut_write();

			// SAFETY: We guarantee that the future is not moved until it's dropped.
			let mut fut = unsafe { Pin::new_unchecked(&mut *fut) };

			// Then poll it, and store the value if finished.
			let new_value = task::ready!(fut.as_mut().poll(cx));
			*self.inner.value.imut_write() = Some(new_value);

			Poll::Ready(())
		})
		.await;

		// Then borrow
		self.inner.waker.trigger.gather_subscribers();
		let borrow = self.inner.value.imut_read();
		BorrowRef(borrow)
	}

	/// Borrows the inner value, without polling the future.
	#[must_use]
	#[track_caller]
	pub fn borrow_suspended(&self) -> Option<BorrowRef<'_, F::Output>> {
		self.inner.waker.trigger.gather_subscribers();

		let borrow = self.inner.value.imut_read();
		borrow.is_some().then(|| BorrowRef(borrow))
	}

	/// Uses the inner value, without polling the future.
	#[track_caller]
	pub fn with_suspended<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Option<&'a F::Output>) -> O,
	{
		let borrow = self.borrow_suspended();
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
		let value = self.inner.value.imut_read();
		f.debug_struct("AsyncSignal").field("value", &value).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T>(IMutRef<'a, Option<T>>);

impl<T> Deref for BorrowRef<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Value wasn't initialized")
	}
}

impl<F: Future> SignalBorrow for AsyncSignal<F> {
	type Ref<'a>
		= Option<BorrowRef<'a, F::Output>>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		// Try to poll the future, if we're not suspended and don't have a value yet
		// TODO: Is it fine to not keep the value locked throughout the poll?
		if !self.is_suspended() && self.inner.value.imut_read().is_none() {
			// Get the inner future through pin projection.
			let mut fut = self.inner.fut.imut_write();

			// SAFETY: We guarantee that the future is not moved until it's dropped.
			let mut fut = unsafe { Pin::new_unchecked(&mut *fut) };

			// Then poll it, and store the value if finished.
			let waker = task::Waker::from(Arc::clone(&self.inner.waker));
			let mut cx = task::Context::from_waker(&waker);
			if let Poll::Ready(new_value) = fut.as_mut().poll(&mut cx) {
				*self.inner.value.imut_write() = Some(new_value);
			}
		}

		self.borrow_suspended()
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
		f(value.as_deref())
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a, T> {
	/// Value
	value: IMutRefMut<'a, Option<T>>,

	/// Trigger on drop
	// Note: Must be dropped *after* `value`.
	_trigger_on_drop: signal::TriggerOnDrop<'a>,
}

impl<T> Deref for BorrowRefMut<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.value.as_ref().expect("Value wasn't initialized")
	}
}

impl<T> DerefMut for BorrowRefMut<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.value.as_mut().expect("Value wasn't initialized")
	}
}

impl<F: Future> SignalBorrowMut for AsyncSignal<F> {
	type RefMut<'a>
		= Option<BorrowRefMut<'a, F::Output>>
	where
		Self: 'a;

	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_> {
		// Note: No need to check if we're suspended, since we doesn't poll here

		let value = self.inner.value.imut_write();
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

	#[track_caller]
	fn update<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut value = self.borrow_mut();
		f(value.as_deref_mut())
	}
}
