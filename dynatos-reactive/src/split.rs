//! Split signals

use {
	crate::{
		Effect,
		EffectRun,
		SignalBorrow,
		SignalBorrowMut,
		SignalGetClonedDefaultImpl,
		SignalGetDefaultImpl,
		SignalSetDefaultImpl,
		SignalUpdateDefaultImpl,
		SignalWithDefaultImpl,
		Trigger,
		effect::EffectSuppressed,
		trigger::TriggerExec,
	},
	core::{
		cell::{self, RefCell},
		clone::Share,
		hash::Hash,
		ops::{self, Deref, DerefMut},
	},
	std::rc::Rc,
	zutil_cloned::cloned,
};

mod hash_map;
mod vec;

pub trait Split {
	type Key: Clone;
	type Value;

	fn index(&self, key: &Self::Key) -> &Self::Value;
	fn try_index(&self, key: &Self::Key) -> Option<&Self::Value>;
	fn iter(&self) -> impl Iterator<Item = (Self::Key, &Self::Value)>;
	fn values(&self) -> impl Iterator<Item = &Self::Value>;
}

impl<S: Split> Split for &S {
	type Key = S::Key;
	type Value = S::Value;

	fn index(&self, key: &Self::Key) -> &Self::Value {
		(**self).index(key)
	}

	fn try_index(&self, key: &Self::Key) -> Option<&Self::Value> {
		(**self).try_index(key)
	}

	fn iter(&self) -> impl Iterator<Item = (Self::Key, &Self::Value)> {
		(**self).iter()
	}

	fn values(&self) -> impl Iterator<Item = &Self::Value> {
		(**self).values()
	}
}

impl<S: Split> Split for &mut S {
	type Key = S::Key;
	type Value = S::Value;

	fn index(&self, key: &Self::Key) -> &Self::Value {
		(**self).index(key)
	}

	fn try_index(&self, key: &Self::Key) -> Option<&Self::Value> {
		(**self).try_index(key)
	}

	fn iter(&self) -> impl Iterator<Item = (Self::Key, &Self::Value)> {
		(**self).iter()
	}

	fn values(&self) -> impl Iterator<Item = &Self::Value> {
		(**self).values()
	}
}

pub trait SplitMut: Split {
	fn index_mut(&mut self, key: &Self::Key) -> &mut Self::Value;
	fn try_index_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Value>;
}

impl<S: SplitMut> SplitMut for &mut S {
	fn index_mut(&mut self, key: &Self::Key) -> &mut Self::Value {
		(**self).index_mut(key)
	}

	fn try_index_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Value> {
		(**self).try_index_mut(key)
	}
}

pub trait SplitStorage: Sized + Split + 'static {
	type Storage<O>: Default + Split<Key = Self::Key, Value = StorageElement<O>>;

	fn sync_storage<O>(&self, storage: &RefCell<Self::Storage<O>>, f: impl Fn(Self::Key, Trigger) -> O);
}


#[derive(derive_more::Debug)]
pub struct StorageElement<O> {
	value:   O,
	trigger: Trigger,
}

#[derive(derive_more::Debug)]
struct Inner<T: SplitStorage, O> {
	storage:        RefCell<T::Storage<O>>,
	update_proxies: Effect,
}

/// Signal split
///
/// Allows splitting a signal of a collection into
/// a collection of signals.
#[derive(derive_more::Debug)]
pub struct SplitSignal<T: SplitStorage, O> {
	inner: Rc<Inner<T, O>>,
}

impl<T: SplitStorage, O> SplitSignal<T, O> {
	pub fn new<S, F>(signal: S, f: F) -> Self
	where
		T::Key: Eq + Hash + Clone + 'static,
		O: 'static,
		S: Clone + for<'a> SignalBorrow<Ref<'a>: Deref<Target = T>> + 'static,
		F: Fn(SplitSignalProxy<S, T, O>) -> O + 'static,
	{
		let inner = Rc::<Inner<T, O>>::new_cyclic(move |inner| {
			#[cloned(inner)]
			let update_proxies = Effect::new_raw(move || {
				let Some(inner) = inner.upgrade() else { return };

				{
					let storage = inner.storage.borrow();
					for (_, value) in storage.iter() {
						// TODO: Should we drop the storage when calling this?
						//       Maybe use a dummy trigger execution to delay this
						//       until after we drop the storage?
						value.trigger.exec();
					}
				}

				signal.borrow().sync_storage(&inner.storage, |key, trigger| {
					f(SplitSignalProxy {
						inner: inner.share(),
						signal: signal.clone(),
						trigger,
						key,
					})
				});
			});

			Inner {
				storage: RefCell::new(T::Storage::default()),
				update_proxies,
			}
		});

		// Note: Needs to run outside because the `Inner` isn't constructed yet.
		inner.update_proxies.run();

		Self { inner }
	}
}

impl<T: SplitStorage, O> SignalBorrow for SplitSignal<T, O> {
	type Ref<'a>
		= SplitSignalBorrow<'a, T, O>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		SplitSignalBorrow(self.inner.storage.borrow())
	}
}

#[derive(derive_more::Debug)]
pub struct SplitSignalBorrow<'a, T: SplitStorage, O>(cell::Ref<'a, T::Storage<O>>);

impl<T: SplitStorage, O> SplitSignalBorrow<'_, T, O> {
	pub fn iter(&self) -> impl Iterator<Item = (T::Key, &O)> {
		self.0.iter().map(|(key, storage)| (key, &storage.value))
	}

	pub fn values(&self) -> impl Iterator<Item = &O> {
		self.0.values().map(|storage| &storage.value)
	}
}

#[derive(Clone, derive_more::Debug)]
pub struct SplitSignalProxy<S, T: SplitStorage, O> {
	inner:   Rc<Inner<T, O>>,
	signal:  S,
	trigger: Trigger,
	key:     T::Key,
}

impl<S, T: SplitStorage, O> SplitSignalProxy<S, T, O> {
	/// Returns the key of this proxy
	pub const fn key(&self) -> &T::Key {
		&self.key
	}
}

impl<S, T: SplitStorage, O> SignalBorrow for SplitSignalProxy<S, T, O>
where
	S: SignalBorrow,
	T::Key: Eq + Hash + Clone,
{
	type Ref<'a>
		= SplitSignalProxyBorrow<'a, S, T>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.trigger.gather_subs();

		SplitSignalProxyBorrow {
			borrow: self.signal.borrow_no_dep(),
			key:    self.key.clone(),
		}
	}
}

impl<S, T: SplitStorage, O> SignalBorrowMut for SplitSignalProxy<S, T, O>
where
	S: SignalBorrowMut,
	T::Key: Eq + Hash + Clone,
{
	type RefMut<'a>
		= SplitSignalProxyBorrowMut<'a, S, T>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		SplitSignalProxyBorrowMut {
			_suppress: self.inner.update_proxies.suppress(),
			borrow:    self.signal.borrow_mut(),
			_exec:     self.trigger.exec(),
			key:       self.key.clone(),
		}
	}
}

impl<S, T: SplitStorage, O> SignalGetDefaultImpl for SplitSignalProxy<S, T, O> {}
impl<S, T: SplitStorage, O> SignalGetClonedDefaultImpl for SplitSignalProxy<S, T, O> {}
impl<S, T: SplitStorage, O> SignalWithDefaultImpl for SplitSignalProxy<S, T, O> {}

impl<S, T: SplitStorage, O> SignalSetDefaultImpl for SplitSignalProxy<S, T, O> {}
impl<S, T: SplitStorage, O> SignalUpdateDefaultImpl for SplitSignalProxy<S, T, O> {}

#[derive(derive_more::Debug)]
pub struct SplitSignalProxyBorrow<'a, S, T: Split>
where
	S: SignalBorrow + 'a,
{
	borrow: S::Ref<'a>,
	key:    T::Key,
}

impl<'a, S, T> ops::Deref for SplitSignalProxyBorrow<'a, S, T>
where
	S: SignalBorrow<Ref<'a>: Deref<Target = T>>,
	T: Split,
	T::Key: Eq + Hash,
{
	type Target = <<S::Ref<'a> as Deref>::Target as Split>::Value;

	fn deref(&self) -> &Self::Target {
		self.borrow.index(&self.key)
	}
}

#[derive(derive_more::Debug)]
pub struct SplitSignalProxyBorrowMut<'a, S, T>
where
	S: SignalBorrowMut + 'a,
	T: Split,
{
	_suppress: EffectSuppressed<'a, dyn EffectRun>,
	borrow:    S::RefMut<'a>,
	_exec:     Option<TriggerExec>,
	key:       T::Key,
}

impl<'a, S, T> ops::Deref for SplitSignalProxyBorrowMut<'a, S, T>
where
	S: SignalBorrowMut<RefMut<'a>: Deref<Target = T>>,
	T: Split,
	T::Key: Eq + Hash,
{
	type Target = <<S::RefMut<'a> as Deref>::Target as Split>::Value;

	fn deref(&self) -> &Self::Target {
		self.borrow.index(&self.key)
	}
}

impl<'a, S, T> ops::DerefMut for SplitSignalProxyBorrowMut<'a, S, T>
where
	S: SignalBorrowMut<RefMut<'a>: DerefMut<Target = T>>,
	T: SplitMut,
	T::Key: Eq + Hash,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.borrow.index_mut(&self.key)
	}
}
