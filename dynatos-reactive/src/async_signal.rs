//! Async signal

// Imports
use {
	crate::{trigger::TriggerWorld, ReactiveWorld, SignalBorrow, SignalWith, Trigger},
	core::{fmt, future::Future, ops::Deref},
	dynatos_world::{IMut, IMutLike, IMutRef, IMutRefMut, IMutRefMutLike, Rc, RcLike, WorldDefault},
	futures::stream::AbortHandle,
	tokio::sync::Notify,
};

/// World for [`AsyncSignal`]
#[expect(private_bounds, reason = "We can't *not* leak some implementation details currently")]
pub trait AsyncSignalWorld<F: Loader> = ReactiveWorld + TriggerWorld where IMut<Inner<F, Self>, Self>: Sized;

/// Inner
struct Inner<F: Loader, W: AsyncSignalWorld<F>> {
	/// Value
	value: Option<F::Output>,

	/// Loader
	loader: F,

	/// Task handle
	handle: Option<AbortHandle>,

	/// Trigger
	trigger: Trigger<W>,

	/// Notify
	notify: Rc<Notify, W>,
}

impl<F: Loader, W: AsyncSignalWorld<F>> Inner<F, W> {
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
	pub fn start_loading(&mut self, this: Rc<IMut<Self, W>, W>) -> bool
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
				this.write().handle = None;
				return;
			};

			// Then write it and remove the handle
			let mut inner = this.write();
			inner.value = Some(value);
			inner.handle = None;
			let trigger = inner.trigger.clone();
			let notify = Rc::<_, W>::clone(&inner.notify);
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
	pub fn restart_loading(&mut self, this: Rc<IMut<Self, W>, W>) -> bool
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
pub struct AsyncSignal<F: Loader, W: AsyncSignalWorld<F> = WorldDefault> {
	/// Inner
	inner: Rc<IMut<Inner<F, W>, W>, W>,
}

impl<F: Loader> AsyncSignal<F, WorldDefault> {
	/// Creates a new async signal with a loader
	#[track_caller]
	#[must_use]
	pub fn new(loader: F) -> Self {
		Self::new_in(loader, WorldDefault::default())
	}
}

impl<F: Loader, W: AsyncSignalWorld<F>> AsyncSignal<F, W> {
	/// Creates a new async signal with a loader in a world
	#[track_caller]
	#[must_use]
	pub fn new_in(loader: F, world: W) -> Self {
		Self {
			inner: Rc::<_, W>::new(IMut::<_, W>::new(Inner {
				value: None,
				loader,
				handle: None,
				trigger: Trigger::new_in(world),
				notify: Rc::<_, W>::new(Notify::new()),
			})),
		}
	}

	/// Stops loading the value.
	///
	/// Returns if the loader had a future.
	pub fn stop_loading(&self) -> bool {
		self.inner.write().stop_loading()
	}

	/// Starts loading the value.
	///
	/// If the loader already has a future, this does nothing.
	///
	/// Returns whether this created the loader's future.
	#[track_caller]
	pub fn start_loading(&self) -> bool
	where
		F: Loader,
	{
		self.inner.write().start_loading(Rc::<_, W>::clone(&self.inner))
	}

	/// Restarts the loading.
	///
	/// If the loader already has a future, it will be dropped
	/// and re-created.
	///
	/// Returns whether a future existed before
	#[track_caller]
	pub fn restart_loading(&self) -> bool
	where
		F: Loader,
	{
		self.inner.write().restart_loading(Rc::<_, W>::clone(&self.inner))
	}

	/// Returns if loading.
	///
	/// This is considered loading if the loader has an active future.
	#[must_use]
	pub fn is_loading(&self) -> bool {
		self.inner.read().is_loading()
	}

	/// Waits for the value to be loaded.
	///
	/// If not loading, waits until the loading starts, but does not start it.
	pub async fn wait(&self) -> BorrowRef<'_, F, W> {
		let inner = self.inner.read();
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
	pub async fn load(&self) -> BorrowRef<'_, F, W> {
		// If the value is loaded, return it
		let mut inner = self.inner.write();
		if inner.value.is_some() {
			return BorrowRef(IMutRefMut::<_, W>::downgrade(inner));
		}

		// Else start loading, and setup a defer to stop loading if we get cancelled.
		// Note: Stopping loading is a no-op if `wait` successfully returns, we only
		//       care if we're dropped early.
		let created_loader = inner.start_loading(Rc::<_, W>::clone(&self.inner));
		scopeguard::defer! {
			if created_loader {
				self.stop_loading();
			}
		}

		// Then wait for the value
		self.wait_inner(IMutRefMut::<_, W>::downgrade(inner)).await
	}

	async fn wait_inner<'a>(&'a self, mut inner: IMutRef<'a, Inner<F, W>, W>) -> BorrowRef<'a, F, W> {
		loop {
			// Register a handle to be notified
			let notify = Rc::<_, W>::clone(&inner.notify);
			let notified = notify.notified();
			drop(inner);

			// Then await on it
			notified.await;

			// Finally return the value
			// Note: If in the meantime the value got overwritten, we wait again
			inner = self.inner.read();
			if inner.value.is_some() {
				break BorrowRef(inner);
			}
		}
	}
}

impl<F: Loader, W: AsyncSignalWorld<F>> Clone for AsyncSignal<F, W> {
	fn clone(&self) -> Self {
		Self {
			inner: Rc::<_, W>::clone(&self.inner),
		}
	}
}

impl<F: Loader, W: AsyncSignalWorld<F>> fmt::Debug for AsyncSignal<F, W>
where
	F::Output: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.borrow();
		f.debug_struct("AsyncSignal").field("value", &value).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, F: Loader, W: AsyncSignalWorld<F> = WorldDefault>(IMutRef<'a, Inner<F, W>, W>);

impl<F: Loader, W: AsyncSignalWorld<F>> fmt::Debug for BorrowRef<'_, F, W>
where
	F::Output: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(**self).fmt(f)
	}
}

impl<F: Loader, W: AsyncSignalWorld<F>> Deref for BorrowRef<'_, F, W> {
	type Target = F::Output;

	fn deref(&self) -> &Self::Target {
		self.0.value.as_ref().expect("Borrow was `None`")
	}
}

impl<F: Loader, W: AsyncSignalWorld<F>> SignalBorrow for AsyncSignal<F, W> {
	type Ref<'a>
		= Option<BorrowRef<'a, F, W>>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		// Start loading on borrow
		let mut inner = self.inner.write();
		inner.start_loading(Rc::<_, W>::clone(&self.inner));

		// Then get the value
		inner
			.value
			.is_some()
			.then(|| BorrowRef(IMutRefMut::<_, W>::downgrade(inner)))
	}
}

impl<F: Loader, W: AsyncSignalWorld<F>> SignalWith for AsyncSignal<F, W>
where
	F::Output: 'static,
{
	type Value<'a> = Option<BorrowRef<'a, F, W>>;

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
