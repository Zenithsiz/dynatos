//! Async signal

// Imports
#[cfg(debug_assertions)]
use core::panic::Location;
use {
	crate::{
		trigger::TriggerExec,
		SignalBorrow,
		SignalBorrowMut,
		SignalGetClone,
		SignalGetClonedDefaultImpl,
		SignalGetCopy,
		SignalGetDefaultImpl,
		SignalSetDefaultImpl,
		SignalUpdate,
		SignalUpdateDefaultImpl,
		SignalWith,
		SignalWithDefaultImpl,
		Trigger,
	},
	core::{
		cell::{self, RefCell},
		fmt,
		future::Future,
		ops::{Deref, DerefMut},
	},
	futures::{future, stream::AbortHandle},
	std::rc::Rc,
	tokio::sync::Notify,
	zutil_cloned::cloned,
};

/// Inner
struct Inner<F: Loader> {
	/// Value
	value: Option<F::Output>,

	/// Loader
	loader: F,

	/// Task handle
	handle: Option<AbortHandle>,
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
	pub fn start_loading(&mut self, signal: &AsyncSignal<F>) -> bool
	where
		F: Loader,
	{
		// If we're loaded, or we're loading, return
		if self.value.is_some() || self.handle.is_some() {
			return false;
		}

		#[cfg(debug_assertions)]
		let caller_loc = Location::caller();

		// Then spawn the future
		// TODO: Allow using something other than `wasm_bindgen_futures`?
		let (fut, handle) = future::abortable(self.loader.load());
		#[cloned(signal)]
		wasm_bindgen_futures::spawn_local(async move {
			// Load the value
			// Note: If we get aborted, just remove the handle
			let Ok(value) = fut.await else {
				signal.inner.borrow_mut().handle = None;
				return;
			};

			// Then write it and remove the handle
			let mut inner = signal.inner.borrow_mut();
			inner.value = Some(value);
			inner.handle = None;
			drop(inner);

			// Finally trigger and awake all waiters.
			// TODO: Notify using the trigger?
			signal.trigger.exec_inner(
				#[cfg(debug_assertions)]
				caller_loc,
			);
			signal.notify.notify_waiters();
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
	pub fn restart_loading(&mut self, signal: &AsyncSignal<F>) -> bool
	where
		F: Loader,
	{
		// cancel the existing future, if any
		let had_fut = self.stop_loading();
		assert!(self.start_loading(signal), "Should start loading");

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
	inner: Rc<RefCell<Inner<F>>>,

	/// Trigger
	trigger: Trigger,

	/// Notify
	notify: Rc<Notify>,
}

impl<F: Loader> AsyncSignal<F> {
	/// Creates a new async signal with a loader
	#[track_caller]
	#[must_use]
	pub fn new(loader: F) -> Self {
		Self {
			inner:   Rc::new(RefCell::new(Inner {
				value: None,
				loader,
				handle: None,
			})),
			trigger: Trigger::new(),
			notify:  Rc::new(Notify::new()),
		}
	}

	/// Stops loading the value.
	///
	/// Returns if the loader had a future.
	#[expect(
		clippy::must_use_candidate,
		reason = "The user may not care whether the future existed"
	)]
	pub fn stop_loading(&self) -> bool {
		self.inner.borrow_mut().stop_loading()
	}

	/// Starts loading the value.
	///
	/// If the loader already has a future, this does nothing.
	///
	/// Returns whether this created the loader's future.
	#[track_caller]
	#[expect(
		clippy::must_use_candidate,
		reason = "The user may not care whether we started the future"
	)]
	pub fn start_loading(&self) -> bool
	where
		F: Loader,
	{
		self.inner.borrow_mut().start_loading(self)
	}

	/// Restarts the loading.
	///
	/// If the loader already has a future, it will be dropped
	/// and re-created.
	///
	/// Returns whether a future existed before
	#[track_caller]
	#[expect(
		clippy::must_use_candidate,
		reason = "The user may not care whether the future existed"
	)]
	pub fn restart_loading(&self) -> bool
	where
		F: Loader,
	{
		self.inner.borrow_mut().restart_loading(self)
	}

	/// Returns if loading.
	///
	/// This is considered loading if the loader has an active future.
	#[must_use]
	pub fn is_loading(&self) -> bool {
		self.inner.borrow().is_loading()
	}

	/// Waits for the value to be loaded.
	///
	/// If not loading, waits until the loading starts, but does not start it.
	pub async fn wait(&self) -> BorrowRef<'_, F> {
		let inner = self.inner.borrow();
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
		#![expect(
			clippy::await_holding_refcell_ref,
			reason = "False positive, we drop it when awaiting"
		)]

		// If the value is loaded, return it
		let mut inner = self.inner.borrow_mut();
		if inner.value.is_some() {
			drop(inner);
			return BorrowRef(self.inner.borrow());
		}

		// Else start loading, and setup a defer to stop loading if we get cancelled.
		// Note: Stopping loading is a no-op if `wait` successfully returns, we only
		//       care if we're dropped early.
		let created_loader = inner.start_loading(self);
		scopeguard::defer! {
			if created_loader {
				self.stop_loading();
			}
		}

		// Then wait for the value
		drop(inner);
		self.wait_inner(self.inner.borrow()).await
	}

	async fn wait_inner<'a>(&'a self, mut inner: cell::Ref<'a, Inner<F>>) -> BorrowRef<'a, F> {
		#![expect(
			clippy::await_holding_refcell_ref,
			reason = "False positive, we drop it when awaiting"
		)]

		loop {
			// Register a handle to be notified
			let notified = self.notify.notified();
			drop(inner);

			// Then await on it
			notified.await;

			// Finally return the value
			// Note: If in the meantime the value got overwritten, we wait again
			inner = self.inner.borrow();
			if inner.value.is_some() {
				break BorrowRef(inner);
			}
		}
	}

	/// Copies the inner value, if loaded.
	///
	/// If unloaded, starts loading it
	#[must_use]
	pub fn get(&self) -> Option<F::Output>
	where
		F::Output: Copy,
	{
		self.borrow().as_deref().copied()
	}

	/// Clones the inner value, if loaded.
	///
	/// If unloaded, starts loading it
	#[must_use]
	pub fn get_cloned(&self) -> Option<F::Output>
	where
		F::Output: Clone,
	{
		self.borrow().as_deref().cloned()
	}

	/// Borrows the value, without loading it
	#[must_use]
	#[track_caller]
	pub fn borrow_unloaded(&self) -> Option<BorrowRef<'_, F>> {
		self.trigger.gather_subscribers();

		self.borrow_unloaded_raw()
	}

	/// Borrows the value, without loading it or gathering subscribers
	#[must_use]
	#[track_caller]
	pub fn borrow_unloaded_raw(&self) -> Option<BorrowRef<'_, F>> {
		let inner = self.inner.borrow();
		inner.value.is_some().then(|| BorrowRef(inner))
	}
}

impl<F: Loader> Clone for AsyncSignal<F> {
	fn clone(&self) -> Self {
		Self {
			inner:   Rc::clone(&self.inner),
			trigger: self.trigger.clone(),
			notify:  Rc::clone(&self.notify),
		}
	}
}

impl<F: Loader> fmt::Debug for AsyncSignal<F>
where
	F::Output: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.borrow_unloaded();
		f.debug_struct("AsyncSignal").field("value", &value.as_deref()).finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
pub struct BorrowRef<'a, F: Loader>(cell::Ref<'a, Inner<F>>);

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

impl<F: Loader<Output: Copy>> SignalGetCopy for Option<BorrowRef<'_, F>> {
	type Value = Option<F::Output>;

	fn copy_value(self) -> Self::Value {
		self.as_deref().copied()
	}
}

impl<F: Loader<Output: Clone>> SignalGetClone for Option<BorrowRef<'_, F>> {
	type Value = Option<F::Output>;

	fn clone_value(self) -> Self::Value {
		self.as_deref().cloned()
	}
}

impl<F: Loader> SignalBorrow for AsyncSignal<F> {
	type Ref<'a>
		= Option<BorrowRef<'a, F>>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.trigger.gather_subscribers();

		self.borrow_raw()
	}

	fn borrow_raw(&self) -> Self::Ref<'_> {
		// Start loading on borrow
		let mut inner = self.inner.borrow_mut();
		inner.start_loading(self);

		// Then get the value
		inner.value.is_some().then(|| {
			drop(inner);
			BorrowRef(self.inner.borrow())
		})
	}
}

impl<F: Loader> SignalWith for AsyncSignal<F>
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

	fn with_raw<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow_raw();
		f(value.as_deref())
	}
}

/// Reference type for [`SignalBorrowMut`] impl
pub struct BorrowRefMut<'a, F: Loader> {
	/// Value
	value: cell::RefMut<'a, Inner<F>>,

	/// Trigger on drop
	// Note: Must be dropped *after* `value`.
	_trigger_on_drop: Option<TriggerExec>,
}

impl<F: Loader> fmt::Debug for BorrowRefMut<'_, F>
where
	F::Output: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		(**self).fmt(f)
	}
}

impl<F: Loader> Deref for BorrowRefMut<'_, F> {
	type Target = F::Output;

	fn deref(&self) -> &Self::Target {
		self.value.value.as_ref().expect("Borrow was `None`")
	}
}

impl<F: Loader> DerefMut for BorrowRefMut<'_, F> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.value.value.as_mut().expect("Borrow was `None`")
	}
}

impl<F: Loader> SignalBorrowMut for AsyncSignal<F> {
	type RefMut<'a>
		= Option<BorrowRefMut<'a, F>>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		// Note: We don't load when mutably borrowing, since that's probably
		//       not what the user wants
		// TODO: Should we even stop loading if the value was set in the meantime?
		let inner = self.inner.borrow_mut();

		// Then get the value
		match inner.value.is_some() {
			true => Some(BorrowRefMut {
				_trigger_on_drop: Some(self.trigger.exec()),
				value:            inner,
			}),
			false => None,
		}
	}

	fn borrow_mut_raw(&self) -> Self::RefMut<'_> {
		// Note: We don't load when mutably borrowing, since that's probably
		//       not what the user wants
		// TODO: Should we even stop loading if the value was set in the meantime?
		let inner = self.inner.borrow_mut();

		// Then get the value
		inner.value.is_some().then(|| BorrowRefMut {
			_trigger_on_drop: None,
			value:            inner,
		})
	}
}

impl<F: Loader> SignalUpdate for AsyncSignal<F>
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

	fn update_raw<F2, O>(&self, f: F2) -> O
	where
		F2: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut value = self.borrow_mut_raw();
		f(value.as_deref_mut())
	}
}

impl<F: Loader> SignalSetDefaultImpl for AsyncSignal<F> {}
impl<F: Loader> SignalGetDefaultImpl for AsyncSignal<F> {}
impl<F: Loader> SignalGetClonedDefaultImpl for AsyncSignal<F> {}

// Note: We want to return an `Option<&T>` instead of `&Option<T>`,
//       so we can't use the default impl
impl<F: Loader> !SignalWithDefaultImpl for AsyncSignal<F> {}
impl<F: Loader> !SignalUpdateDefaultImpl for AsyncSignal<F> {}

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
