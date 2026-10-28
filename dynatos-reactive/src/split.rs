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

/// A collection for [`SplitSignal`].
///
/// Requires the collection to be indexable with a key ([`Self::Key`]).
/// For example, [`Vec`] uses it's index as the key, while [`HashMap`](std::collections::HashMap)
/// uses the actual key.
///
/// Collections must also be iterable.
/// Both [`Self::iter`] and [`Self::values`] are requires because cloning the
/// keys might not be free (like it is in `Vec`) when you only need the values.
///
/// We also can't return references to the keys because [`Vec`] is unable to do so,
/// given that it's keys (indices) aren't actually stored anywhere.
pub trait Collection {
	type Key: Clone;
	type Value;

	fn index(&self, key: &Self::Key) -> &Self::Value;
	fn iter(&self) -> impl Iterator<Item = (Self::Key, &Self::Value)>;
	fn values(&self) -> impl Iterator<Item = &Self::Value>;
}

impl<S: Collection> Collection for &S {
	type Key = S::Key;
	type Value = S::Value;

	fn index(&self, key: &Self::Key) -> &Self::Value {
		(**self).index(key)
	}

	fn iter(&self) -> impl Iterator<Item = (Self::Key, &Self::Value)> {
		(**self).iter()
	}

	fn values(&self) -> impl Iterator<Item = &Self::Value> {
		(**self).values()
	}
}

impl<S: Collection> Collection for &mut S {
	type Key = S::Key;
	type Value = S::Value;

	fn index(&self, key: &Self::Key) -> &Self::Value {
		(**self).index(key)
	}

	fn iter(&self) -> impl Iterator<Item = (Self::Key, &Self::Value)> {
		(**self).iter()
	}

	fn values(&self) -> impl Iterator<Item = &Self::Value> {
		(**self).values()
	}
}

/// A mutable collection for [`SplitSignal`]
///
/// See [`Collection`] for more details.
///
/// This only enhances it with mutably indexing.
pub trait CollectionMut: Collection {
	fn index_mut(&mut self, key: &Self::Key) -> &mut Self::Value;
}

impl<S: CollectionMut> CollectionMut for &mut S {
	fn index_mut(&mut self, key: &Self::Key) -> &mut Self::Value {
		(**self).index_mut(key)
	}
}

/// A collection with storage for [`SplitSignal`]
///
/// This trait defines how [`SplitSignal`] stores the values.
/// In particular, [`SplitSignal`] needs to store a collection of
/// [`StorageElement`]s, which is defined by [`Self::Storage`].
///
/// [`Self::sync_storage`] serves to synchronize this collection with the storage,
/// ensuring that any keys that no longer exist are removed from storage and any new
/// keys are created.
pub trait CollectionStorage: Collection + 'static {
	type Storage<O>: Default + Collection<Key = Self::Key, Value = StorageElement<O>>;

	fn sync_storage<O>(&self, storage: &RefCell<Self::Storage<O>>, f: impl Fn(Self::Key, Trigger) -> O);
}

/// A storage element.
///
/// Just an implementation detail for [`SplitSignal`].
/// When implementing [`CollectionStorage`] for your own types,
/// you can create new elements with [`Self::new`].
#[derive(derive_more::Debug)]
pub struct StorageElement<O> {
	value:   O,
	trigger: Trigger,
}

impl<O> StorageElement<O> {
	pub const fn new(value: O, trigger: Trigger) -> Self {
		Self { value, trigger }
	}
}

#[derive(derive_more::Debug)]
struct Inner<T: CollectionStorage, O> {
	storage:        RefCell<T::Storage<O>>,
	update_proxies: Effect,
}

/// Signal split
///
/// Allows splitting a signal of a collection into
/// a collection of signals.
///
/// The collection of signals is special, in that writes through
/// the "proxy signal" (i.e. the signal inside of the collection of signals)
/// will update all dependencies of the original signals, *except* for other
/// proxy signals, reducing unnecessary work.
///
/// # Examples
///
/// Simple example demonstrating how to split a `Signal<Vec<_>>`
/// into a `impl Iterator<Item = Derived<_, _>>`.
///
/// ```
/// # #![feature(proc_macro_hygiene, stmt_expr_attributes)]
/// # use {dynatos_reactive::{Signal, Derived, SplitSignal, SignalGet, SignalBorrow, SignalSet}, zutil_cloned::cloned};
/// let signal = Signal::new(vec![1, 2, 3]);
///
/// // To create a split signal, you pass in your signal and
/// // a function that creates signals from a proxy signal.
/// let split_signal = SplitSignal::new(signal.clone(), move |proxy| {
/// 	Derived::new(move || proxy.get() + 2).unsize()
/// });
///
/// // You can access the split signal to get all of the signals.
/// assert_eq!(split_signal.borrow().values().map(|derived| derived.get()).collect::<Vec<_>>(), [3, 4, 5]);
///
/// // Modifying the outer signal modifies all inner signals
/// signal.set(vec![2, 3, 4]);
/// assert_eq!(split_signal.borrow().values().map(|derived| derived.get()).collect::<Vec<_>>(), [4, 5, 6]);
/// ```
#[derive(derive_more::Debug)]
pub struct SplitSignal<T: CollectionStorage, O> {
	inner: Rc<Inner<T, O>>,
}

impl<T: CollectionStorage, O> SplitSignal<T, O> {
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

impl<T: CollectionStorage, O> SignalBorrow for SplitSignal<T, O> {
	type Ref<'a>
		= SplitSignalBorrow<'a, T, O>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		SplitSignalBorrow(self.inner.storage.borrow())
	}
}

#[derive(derive_more::Debug)]
pub struct SplitSignalBorrow<'a, T: CollectionStorage, O>(cell::Ref<'a, T::Storage<O>>);

impl<T: CollectionStorage, O> SplitSignalBorrow<'_, T, O> {
	pub fn iter(&self) -> impl Iterator<Item = (T::Key, &O)> {
		self.0.iter().map(|(key, storage)| (key, &storage.value))
	}

	pub fn values(&self) -> impl Iterator<Item = &O> {
		self.0.values().map(|storage| &storage.value)
	}
}

#[derive(Clone, derive_more::Debug)]
pub struct SplitSignalProxy<S, T: CollectionStorage, O> {
	inner:   Rc<Inner<T, O>>,
	signal:  S,
	trigger: Trigger,
	key:     T::Key,
}

impl<S, T: CollectionStorage, O> SplitSignalProxy<S, T, O> {
	/// Returns the key of this proxy
	pub const fn key(&self) -> &T::Key {
		&self.key
	}
}

impl<S, T: CollectionStorage, O> SignalBorrow for SplitSignalProxy<S, T, O>
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

impl<S, T: CollectionStorage, O> SignalBorrowMut for SplitSignalProxy<S, T, O>
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

impl<S, T: CollectionStorage, O> SignalGetDefaultImpl for SplitSignalProxy<S, T, O> {}
impl<S, T: CollectionStorage, O> SignalGetClonedDefaultImpl for SplitSignalProxy<S, T, O> {}
impl<S, T: CollectionStorage, O> SignalWithDefaultImpl for SplitSignalProxy<S, T, O> {}

impl<S, T: CollectionStorage, O> SignalSetDefaultImpl for SplitSignalProxy<S, T, O> {}
impl<S, T: CollectionStorage, O> SignalUpdateDefaultImpl for SplitSignalProxy<S, T, O> {}

#[derive(derive_more::Debug)]
pub struct SplitSignalProxyBorrow<'a, S, T: Collection>
where
	S: SignalBorrow + 'a,
{
	borrow: S::Ref<'a>,
	key:    T::Key,
}

impl<'a, S, T> ops::Deref for SplitSignalProxyBorrow<'a, S, T>
where
	S: SignalBorrow<Ref<'a>: Deref<Target = T>>,
	T: Collection,
	T::Key: Eq + Hash,
{
	type Target = <<S::Ref<'a> as Deref>::Target as Collection>::Value;

	fn deref(&self) -> &Self::Target {
		self.borrow.index(&self.key)
	}
}

#[derive(derive_more::Debug)]
pub struct SplitSignalProxyBorrowMut<'a, S, T>
where
	S: SignalBorrowMut + 'a,
	T: Collection,
{
	_suppress: EffectSuppressed<'a, dyn EffectRun>,
	borrow:    S::RefMut<'a>,
	_exec:     Option<TriggerExec>,
	key:       T::Key,
}

impl<'a, S, T> ops::Deref for SplitSignalProxyBorrowMut<'a, S, T>
where
	S: SignalBorrowMut<RefMut<'a>: Deref<Target = T>>,
	T: Collection,
	T::Key: Eq + Hash,
{
	type Target = <<S::RefMut<'a> as Deref>::Target as Collection>::Value;

	fn deref(&self) -> &Self::Target {
		self.borrow.index(&self.key)
	}
}

impl<'a, S, T> ops::DerefMut for SplitSignalProxyBorrowMut<'a, S, T>
where
	S: SignalBorrowMut<RefMut<'a>: DerefMut<Target = T>>,
	T: CollectionMut,
	T::Key: Eq + Hash,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.borrow.index_mut(&self.key)
	}
}
