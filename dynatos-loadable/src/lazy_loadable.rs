//! Lazy loadable.

// Imports
use {
	crate::Loadable,
	dynatos_reactive::{Effect, Signal, SignalGet, SignalSet, SignalUpdate, SignalWith},
	std::{future::Future, rc::Rc},
};

/// Load status
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum LoadStatus {
	/// Not loading
	Unloaded,

	/// Load if empty
	LoadIfEmpty,

	/// Always load
	LoadAlways,

	/// Loading
	Loading,
}

impl LoadStatus {
	/// Sets this load status, to at least `load_status`.
	///
	/// If the current value is higher than `load_status`, this value won't
	/// be lowered.
	pub fn set_at_least(&mut self, other: Self) {
		*self = Ord::max(*self, other);
	}
}

/// A lazy loadable.
#[derive(Debug)]
pub struct LazyLoadable<T, E> {
	/// Inner
	inner: Signal<Loadable<T, E>>,

	/// Load status
	load_status: Signal<LoadStatus>,

	/// Effect
	effect: Effect,
}

impl<T, E> LazyLoadable<T, E> {
	/// Creates a new, empty, loadable.
	pub fn new<F, Fut>(load: F) -> Self
	where
		T: 'static,
		E: 'static,
		F: Fn() -> Fut + 'static,
		Fut: Future<Output = Result<T, E>> + 'static,
	{
		let inner = Signal::new(Loadable::Empty);
		let load_status = Signal::new(LoadStatus::Unloaded);
		let load = Rc::new(load);

		let effect = Effect::new({
			let inner = inner.clone();
			let load_status = load_status.clone();
			move || {
				// If we're loading, or shouldn't load, quit.
				let should_load = match load_status.get() {
					LoadStatus::Unloaded => false,
					LoadStatus::LoadIfEmpty => inner.with(|inner| inner.is_empty()),
					LoadStatus::LoadAlways => true,
					LoadStatus::Loading => false,
				};
				if !should_load {
					return;
				}

				// Else spawn a future to load the value
				// Note: We need to ensure that we only update `inner` inside while `load_status`
				//       is `Loading`. Otherwise, we could get a race and lose the loaded value to
				//       another load.
				// Note: Although we could get the future outside of `spawn_local`, if we do, it's
				//       dependencies would leak into this effect, which we don't want. This way, the
				//       user also receives a warning if they try to use any dependencies within `load`.
				load_status.set(LoadStatus::Loading);
				let inner = inner.clone();
				let load_status = load_status.clone();
				let load = Rc::clone(&load);
				wasm_bindgen_futures::spawn_local(async move {
					let res = load().await;
					inner.set(Loadable::from_res(res));
					load_status.set(LoadStatus::Unloaded);
				});
			}
		});

		Self {
			inner,
			load_status,
			effect,
		}
	}

	/// Starts loading.
	///
	/// If loading or loaded, does nothing
	pub fn load(&self) {
		self.load_status
			.update(|load_status| load_status.set_at_least(LoadStatus::LoadIfEmpty));
	}

	/// Reloads the inner value.
	///
	/// If loading, does nothing.
	pub fn reload(&self) {
		self.load_status
			.update(|load_status| load_status.set_at_least(LoadStatus::LoadAlways));
	}

	/// Reactively accesses the value, without loading it.
	pub fn with_unloaded<R>(&self, f: impl FnOnce(Loadable<&T, E>) -> R) -> R
	where
		E: Clone,
	{
		self.inner.with(|value| f(value.as_ref()))
	}

	/// Updates the value mutably, without loading it.
	///
	/// Will notify any subscribers of the value.
	pub fn update_unloaded<R>(&self, f: impl FnOnce(Loadable<&mut T, E>) -> R) -> R
	where
		E: Clone,
	{
		let mut output = None;
		self.inner.update(|inner| output = Some(f(inner.as_mut())));

		output.expect("Value was not updated")
	}
}

impl<T, E> Clone for LazyLoadable<T, E> {
	fn clone(&self) -> Self {
		Self {
			inner:       self.inner.clone(),
			load_status: self.load_status.clone(),
			effect:      self.effect.clone(),
		}
	}
}

// TODO: Use a `Loadable<&T, E>` when `SignalWith` allows?
impl<T, E> SignalWith for LazyLoadable<T, E>
where
	E: Clone,
{
	type Value = Loadable<T, E>;

	fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Self::Value) -> O,
	{
		self.load();
		self.inner.with(f)
	}
}

impl<T, E> SignalUpdate for LazyLoadable<T, E>
where
	E: Clone,
{
	type Value = Loadable<T, E>;

	fn update<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&mut Self::Value) -> O,
	{
		self.load();
		self.inner.update(f)
	}
}
