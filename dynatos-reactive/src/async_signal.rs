//! Async signal

// Imports
use {
	crate::{
		effect,
		loc::Loc,
		trigger::TriggerExec,
		world::UnloadedGuard,
		Effect,
		EffectRun,
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
		WORLD,
	},
	core::{
		cell::{self, RefCell},
		fmt,
		future::Future,
		ops::{Deref, DerefMut},
	},
	futures::{future, stream::AbortHandle},
	std::rc::Rc,
	zutil_cloned::cloned,
};

/// Inner
struct Inner<F: Loader> {
	/// Value
	value: Option<F::Output>,

	/// Loader
	loader: F,

	/// Restart effect
	// TODO: Not have this in an option just to be able to initialize it
	restart_effect: Option<Effect<RestartEffectFn<F>>>,

	/// Task handle
	handle: Option<AbortHandle>,
}

/// Type for [`Inner::restart_effect`]'s effect
type RestartEffectFn<F: Loader> = impl EffectRun;

impl<F: Loader> Inner<F> {
	/// See [`AsyncSignal::stop_loading`]
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

	/// See [`AsyncSignal::start_loading`]
	#[track_caller]
	pub fn start_loading(&mut self, signal: &AsyncSignal<F>) -> bool
	where
		F: Loader,
	{
		// If we're already loading, return
		if self.is_loading() {
			return false;
		}

		let caller_loc = Loc::caller();

		// Then spawn the future
		// TODO: Allow using something other than `wasm_bindgen_futures`?
		let restart_effect = self.restart_effect.clone().expect("Missing restart effect");
		let fut = restart_effect.gather_deps(|| self.loader.load());
		let (fut, handle) = future::abortable(fut);
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

			// Finally trigger
			let _suppress_restart = restart_effect.suppress();
			signal.trigger.exec_inner(caller_loc);
		});
		self.handle = Some(handle);

		true
	}

	/// See [`AsyncSignal::restart_loading`]
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

	/// See [`AsyncSignal::is_loading`]
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
}

impl<F: Loader> AsyncSignal<F> {
	/// Creates a new async signal with a reactive loader.
	///
	/// When any inputs to `loader` change, the signal's
	/// future will be restarted.
	#[track_caller]
	#[must_use]
	#[define_opaque(RestartEffectFn)]
	pub fn new(loader: F) -> Self {
		let signal = Self {
			inner:   Rc::new(RefCell::new(Inner {
				value: None,
				loader,
				restart_effect: None,
				handle: None,
			})),
			trigger: Trigger::new(),
		};

		signal.inner.borrow_mut().restart_effect = Some(Effect::<RestartEffectFn<F>>::new_raw(
			#[cloned(signal)]
			move || {
				// TODO: Fix the exec location coming from here
				signal.restart_loading();
			},
		));

		signal
	}

	/// Stops the loading future.
	///
	/// Returns if any future existed.
	#[expect(
		clippy::must_use_candidate,
		reason = "The user may not care whether the future existed"
	)]
	pub fn stop_loading(&self) -> bool {
		self.inner.borrow_mut().stop_loading()
	}

	/// Starts a new loading future.
	///
	/// If a future already exists, this does nothing.
	///
	/// If a value already exists, this won't remove it, but
	/// will overwrite it once the future completes.
	///
	/// Returns whether we created a new future.
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

	/// Restarts the currently loading future.
	///
	/// If a future already exists, this will stop it and begin a
	/// new one.
	///
	/// If a value already exists, this won't remove it, but
	/// will overwrite it once the future completes.
	///
	/// Returns whether a future already existed.
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

	/// Returns if there exists a loading future.
	#[must_use]
	pub fn is_loading(&self) -> bool {
		self.inner.borrow().is_loading()
	}

	/// Borrows the value, without loading it
	#[must_use]
	#[track_caller]
	pub fn borrow_unloaded(&self) -> Option<BorrowRef<'_, F>> {
		self::with_unloaded(|| self.borrow())
	}

	/// Borrows the value, without loading it or gathering subscribers
	#[must_use]
	#[track_caller]
	pub fn borrow_unloaded_raw(&self) -> Option<BorrowRef<'_, F>> {
		effect::with_raw(|| self.borrow_unloaded())
	}
}

impl<F: Loader> Clone for AsyncSignal<F> {
	fn clone(&self) -> Self {
		Self {
			inner:   Rc::clone(&self.inner),
			trigger: self.trigger.clone(),
		}
	}
}

#[coverage(off)]
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

#[coverage(off)]
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
		self.trigger.gather_subs();

		let inner = self.inner.borrow();
		match &inner.value {
			// If there's already a value, return it
			Some(_) => Some(BorrowRef(inner)),

			// Otherwise, start loading if not in "unloaded" mode
			None => {
				if WORLD.is_unloaded() {
					return None;
				}

				drop(inner);
				self.inner.borrow_mut().start_loading(self);
				None
			},
		}
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
}

/// Reference type for [`SignalBorrowMut`] impl
pub struct BorrowRefMut<'a, F: Loader> {
	/// Value
	value: cell::RefMut<'a, Inner<F>>,

	/// Trigger on drop
	// Note: Must be dropped *after* `value`.
	_trigger_on_drop: Option<TriggerExec>,
}

#[coverage(off)]
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
				_trigger_on_drop: self.trigger.exec(),
				value:            inner,
			}),
			false => None,
		}
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

/// Enters "unloaded" mode within the supplied closure.
///
/// Within "unloaded" mode, async signals will not start loading
/// their contents when accessed.
#[track_caller]
pub fn with_unloaded<F, O>(f: F) -> O
where
	F: FnOnce() -> O,
{
	let _guard = WORLD.set_unloaded();
	f()
}

/// Enters "unloaded" mode with a guard
///
/// See [`with_unloaded`] for details.
pub fn enter_unloaded() -> UnloadedGuard {
	WORLD.set_unloaded()
}

/// Returns if "unloaded" mode is on
pub fn is_unloaded() -> bool {
	WORLD.is_unloaded()
}
