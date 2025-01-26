//! Async signal

// Imports
use {
	core::{fmt, future::Future, ops::Deref},
	dynatos_reactive::{SignalBorrow, SignalWith, Trigger},
	dynatos_reactive_sync::{IMut, IMutExt, IMutRef, IMutRefMut, IMutRefMutExt, Rc},
	futures::stream::AbortHandle,
	tokio::sync::Notify,
};

/// Inner
struct Inner<F: Loader> {
	/// Value
	value: Option<F::Output>,

	/// Loader
	loader: F,

	/// Task handle
	handle: Option<AbortHandle>,

	/// Trigger
	trigger: Trigger,

	/// Notify
	notify: Rc<Notify>,
}

impl<F: Loader> Inner<F> {
	/// Stops loading the value.
	///
	/// Returns if the loader had a future.
	pub fn stop_loading(&mut self) -> bool {
		let handle = self.handle.take();
		match handle {
			Some(handle) => {
				handle.abort();
				true
			},
			None => false,
		}
	}

	/// Starts loading the value.
	///
	/// If the loader already has a future, this does nothing.
	///
	/// Returns whether this created the loader's future.
	#[track_caller]
	pub fn start_loading(&mut self, this: Rc<IMut<Self>>) -> bool
	where
		F: Loader,
	{
		// If we're loaded, or we're loading, return
		if self.value.is_some() || self.handle.is_some() {
			return false;
		}

		// Gather subscribers when loading
		self.trigger.gather_subscribers();

		// Then spawn the future
		// TODO: Allow using something other than `wasm_bindgen_futures`?
		let (fut, handle) = futures::future::abortable(self.loader.load());
		wasm_bindgen_futures::spawn_local(async move {
			// Load the value
			// Note: If we get aborted, just remove the handle
			let Ok(value) = fut.await else {
				this.imut_write().handle = None;
				return;
			};

			// Then write it and remove the handle
			let mut inner = this.imut_write();
			inner.value = Some(value);
			inner.handle = None;
			let trigger = inner.trigger.clone();
			let notify = Rc::clone(&inner.notify);
			drop(inner);

			// Finally trigger and awake all waiters.
			// TODO: Notify using the trigger?
			trigger.trigger();
			notify.notify_waiters();
		});
		self.handle = Some(handle);

		true
	}

	/// Restarts the loading.
	///
	/// If the loader already has a future, it will be dropped
	/// and re-created.
	///
	/// Returns whether a future existed before
	#[track_caller]
	pub fn restart_loading(&mut self, this: Rc<IMut<Self>>) -> bool
	where
		F: Loader,
	{
		// cancel the existing future, if any
		let had_fut = self.stop_loading();
		assert!(self.start_loading(this), "Should start loading");

		had_fut
	}

	/// Returns if loading.
	///
	/// This is considered loading if the loader has an active future.
	#[must_use]
	pub const fn is_loading(&self) -> bool {
		self.handle.is_some()
	}
}

/// Async signal
pub struct AsyncSignal<F: Loader> {
	/// Inner
	inner: Rc<IMut<Inner<F>>>,
}

impl<F: Loader> AsyncSignal<F> {
	/// Creates a new async signal with a loader
	#[track_caller]
	#[must_use]
	pub fn new(loader: F) -> Self {
		Self {
			inner: Rc::new(IMut::new(Inner {
				value: None,
				loader,
				handle: None,
				trigger: Trigger::new(),
				notify: Rc::new(Notify::new()),
			})),
		}
	}

	/// Stops loading the value.
	///
	/// Returns if the loader had a future.
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	pub fn stop_loading(&self) -> bool {
		self.inner.imut_write().stop_loading()
	}

	/// Starts loading the value.
	///
	/// If the loader already has a future, this does nothing.
	///
	/// Returns whether this created the loader's future.
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	#[track_caller]
	pub fn start_loading(&self) -> bool
	where
		F: Loader,
	{
		self.inner.imut_write().start_loading(Rc::clone(&self.inner))
	}

	/// Restarts the loading.
	///
	/// If the loader already has a future, it will be dropped
	/// and re-created.
	///
	/// Returns whether a future existed before
	#[expect(clippy::must_use_candidate, reason = "It's fine to ignore")]
	#[track_caller]
	pub fn restart_loading(&self) -> bool
	where
		F: Loader,
	{
		self.inner.imut_write().restart_loading(Rc::clone(&self.inner))
	}

	/// Returns if loading.
	///
	/// This is considered loading if the loader has an active future.
	#[must_use]
	pub fn is_loading(&self) -> bool {
		self.inner.imut_read().is_loading()
	}

	/// Waits for the value to be loaded.
	///
	/// If not loading, waits until the loading starts, but does not start it.
	pub async fn wait(&self) -> BorrowRef<'_, F> {
		let inner = self.inner.imut_read();
		self.wait_inner(inner).await
	}

	/// Loads the inner value.
	///
	/// If already loaded, returns it without loading.
	///
	/// Otherwise, this will start loading.
	///
	/// If this future is dropped before completion, the loading
	/// will be cancelled.
	pub async fn load(&self) -> BorrowRef<'_, F> {
		// If the value is loaded, return it
		let mut inner = self.inner.imut_write();
		if inner.value.is_some() {
			return BorrowRef(IMutRefMut::imut_downgrade(inner));
		}

		// Else start loading, and setup a defer to stop loading if we get cancelled.
		// Note: Stopping loading is a no-op if `wait` successfully returns, we only
		//       care if we're dropped early.
		let created_loader = inner.start_loading(Rc::clone(&self.inner));
		scopeguard::defer! {
			if created_loader {
				self.stop_loading();
			}
		}

		// Then wait for the value
		self.wait_inner(IMutRefMut::imut_downgrade(inner)).await
	}

	#[expect(clippy::await_holding_refcell_ref, reason = "We drop it when awaiting it")]
	async fn wait_inner<'a>(&'a self, mut inner: IMutRef<'a, Inner<F>>) -> BorrowRef<'a, F> {
		loop {
			// Register a handle to be notified
			let notify = Rc::clone(&inner.notify);
			let notified = notify.notified();
			drop(inner);

			// Then await on it
			notified.await;

			// Finally return the value
			// Note: If in the meantime the value got overwritten, we wait again
			inner = self.inner.imut_read();
			if inner.value.is_some() {
				break BorrowRef(inner);
			}
		}
	}
}

impl<F: Loader> Clone for AsyncSignal<F> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::clone(&self.inner),
		}
	}
}

impl<F: Loader> fmt::Debug for AsyncSignal<F>
where
	F::Output: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.borrow();
		f.debug_struct("AsyncSignal").field("value", &value).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, F: Loader>(IMutRef<'a, Inner<F>>);

impl<F: Loader> fmt::Debug for BorrowRef<'_, F>
where
	F::Output: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(**self).fmt(f)
	}
}

impl<F: Loader> Deref for BorrowRef<'_, F> {
	type Target = F::Output;

	fn deref(&self) -> &Self::Target {
		self.0.value.as_ref().expect("Borrow was `None`")
	}
}

impl<F: Loader> SignalBorrow for AsyncSignal<F> {
	type Ref<'a>
		= Option<BorrowRef<'a, F>>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		// Start loading on borrow
		let mut inner = self.inner.imut_write();
		inner.start_loading(Rc::clone(&self.inner));

		// Then get the value
		inner
			.value
			.is_some()
			.then(|| BorrowRef(IMutRefMut::imut_downgrade(inner)))
	}
}

impl<F: Loader> SignalWith for AsyncSignal<F>
where
	F::Output: 'static,
{
	type Value<'a> = Option<BorrowRef<'a, F>>;

	#[track_caller]
	fn with<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(value)
	}
}

/// Loader
pub trait Loader: 'static {
	type Fut: Future<Output = Self::Output> + 'static;
	type Output;

	fn load(&mut self) -> Self::Fut;
}

impl<F> Loader for F
where
	F: FnMut<()> + 'static,
	F::Output: Future + 'static,
{
	type Fut = F::Output;
	type Output = <F::Output as Future>::Output;

	fn load(&mut self) -> Self::Fut {
		(self)()
	}
}
