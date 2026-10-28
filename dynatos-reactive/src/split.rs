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
		cell::{self, OnceCell, RefCell},
		clone::Share,
		hash::{BuildHasher, Hash},
		ops::{self, Deref, DerefMut},
	},
	std::{
		collections::{HashMap, hash_map},
		rc::Rc,
	},
	zutil_cloned::cloned,
};

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

	fn sync_storage<O>(&self, storage: &RefCell<Self::Storage<O>>, f: impl Fn(Self::Key) -> O);
}


impl<T> Split for Vec<T> {
	type Key = usize;
	type Value = T;

	fn index(&self, key: &Self::Key) -> &Self::Value {
		&self[*key]
	}

	fn try_index(&self, key: &Self::Key) -> Option<&Self::Value> {
		self.get(*key)
	}

	fn iter(&self) -> impl Iterator<Item = (Self::Key, &Self::Value)> {
		self.as_slice().iter().enumerate()
	}

	fn values(&self) -> impl Iterator<Item = &Self::Value> {
		self.as_slice().iter()
	}
}

impl<T> SplitMut for Vec<T> {
	fn index_mut(&mut self, key: &Self::Key) -> &mut Self::Value {
		&mut self[*key]
	}

	fn try_index_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Value> {
		self.get_mut(*key)
	}
}

impl<T: 'static> SplitStorage for Vec<T> {
	type Storage<O> = Vec<StorageElement<O>>;

	fn sync_storage<O>(&self, storage: &RefCell<Self::Storage<O>>, f: impl Fn(Self::Key) -> O) {
		let mut storage_borrow = storage.borrow_mut();
		match self.len() <= storage_borrow.len() {
			true => storage_borrow.truncate(self.len()),
			false =>
				for idx in storage_borrow.len()..self.len() {
					storage_borrow.push(StorageElement::default());

					drop(storage_borrow);
					let value = f(idx);

					storage_borrow = storage.borrow_mut();

					assert!(storage_borrow[idx].value.set(value).is_ok(), "Value was initialized");
				},
		}
	}
}


impl<K, V, S> Split for HashMap<K, V, S>
where
	K: Clone + Eq + Hash,
	S: BuildHasher,
{
	type Key = K;
	type Value = V;

	fn index(&self, key: &Self::Key) -> &Self::Value {
		&self[key]
	}

	fn try_index(&self, key: &Self::Key) -> Option<&Self::Value> {
		self.get(key)
	}

	fn iter(&self) -> impl Iterator<Item = (Self::Key, &Self::Value)> {
		self.iter().map(|(key, value)| (key.clone(), value))
	}

	fn values(&self) -> impl Iterator<Item = &Self::Value> {
		self.values()
	}
}

impl<K, V, S> SplitMut for HashMap<K, V, S>
where
	K: Clone + Eq + Hash,
	S: BuildHasher,
{
	fn index_mut(&mut self, key: &Self::Key) -> &mut Self::Value {
		self.get_mut(key).expect("Invalid key")
	}

	fn try_index_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Value> {
		self.get_mut(key)
	}
}

impl<K, V, HS> SplitStorage for HashMap<K, V, HS>
where
	K: Clone + Eq + Hash + 'static,
	V: 'static,
	HS: BuildHasher + 'static,
{
	type Storage<O> = HashMap<K, StorageElement<O>>;

	fn sync_storage<O>(&self, storage: &RefCell<Self::Storage<O>>, f: impl Fn(Self::Key) -> O) {
		let mut storage_borrow = storage.borrow_mut();

		// TODO: Not have to go over both maps to add/remove?
		storage_borrow.retain(|key, _| self.contains_key(key));
		#[expect(clippy::iter_over_hash_type, reason = "Order doesn't matter")]
		for key in self.keys() {
			let hash_map::Entry::Vacant(entry) = storage_borrow.entry(key.clone()) else {
				continue;
			};

			entry.insert(StorageElement::default());
			drop(storage_borrow);

			let value = f(key.clone());
			storage_borrow = storage.borrow_mut();
			assert!(
				storage_borrow
					.get_mut(key)
					.expect("Should exist")
					.value
					.set(value)
					.is_ok(),
				"Value was initialized"
			);
		}
	}
}

#[derive(derive_more::Debug)]
pub struct StorageElement<O> {
	value:   OnceCell<O>,
	trigger: Trigger,
}

impl<O> Default for StorageElement<O> {
	fn default() -> Self {
		Self {
			value:   OnceCell::new(),
			trigger: Trigger::new(),
		}
	}
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

				signal.borrow().sync_storage(&inner.storage, |key| {
					let proxy = SplitSignalProxy {
						inner: inner.share(),
						signal: signal.clone(),
						key,
					};
					f(proxy)
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
		self.0.iter().map(|(key, storage)| {
			let value = storage.value.get().expect("Value was missing");
			(key, value)
		})
	}

	pub fn values(&self) -> impl Iterator<Item = &O> {
		self.0
			.values()
			.map(|storage| storage.value.get().expect("Value was missing"))
	}
}

#[derive(Clone, derive_more::Debug)]
pub struct SplitSignalProxy<S, T: SplitStorage, O> {
	inner:  Rc<Inner<T, O>>,
	signal: S,
	key:    T::Key,
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
		self.inner.storage.borrow().index(&self.key).trigger.gather_subs();

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
		let exec = self.inner.storage.borrow().index(&self.key).trigger.exec();

		SplitSignalProxyBorrowMut {
			_suppress: self.inner.update_proxies.suppress(),
			borrow:    self.signal.borrow_mut(),
			_exec:     exec,
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

#[cfg(test)]
mod tests {
	use {
		crate::{Derived, Signal, SignalBorrow, SignalBorrowMut, SignalGet, SignalSet, Trigger, split::SplitSignal},
		core::cell::{Cell, RefCell},
		itertools::Itertools,
		std::{collections::HashMap, hash_map, rc::Rc},
		zutil_cloned::cloned,
	};

	#[test]
	fn vec() {
		let signal = Signal::new(vec![1, 2, 3]);

		let outer_derived_updated = Rc::new(Cell::new(0_usize));

		#[cloned(signal, outer_derived_updated)]
		let get_values = Derived::new(move || {
			outer_derived_updated.update(|value| value + 1);
			signal.borrow().iter().copied().collect::<Vec<_>>()
		});

		let inner_derived_updated = Rc::new(Cell::new(0_usize));
		let inner_proxies = Rc::new(RefCell::new(vec![]));

		#[cloned(inner_derived_updated, inner_proxies)]
		let split_signal = SplitSignal::new(signal.clone(), move |proxy| {
			inner_proxies.borrow_mut().push(proxy.clone());

			#[cloned(inner_derived_updated)]
			Derived::new(move || {
				inner_derived_updated.update(|value| value + 1);
				proxy.get() + 2
			})
			.unsize()
		});


		let get_derived_values = move || {
			split_signal
				.borrow()
				.values()
				.map(|derived| derived.get())
				.collect::<Vec<_>>()
		};

		assert_eq!(outer_derived_updated.get(), 1);
		assert_eq!(inner_derived_updated.get(), 3);
		assert_eq!(inner_proxies.borrow().len(), 3);
		assert_eq!(&**get_values.borrow(), [1, 2, 3]);
		assert_eq!(get_derived_values(), [3, 4, 5]);

		// Updating through the original signal must update all derived without
		// re-creating them.
		signal.borrow_mut()[0] = 4;
		assert_eq!(outer_derived_updated.get(), 2);
		assert_eq!(inner_derived_updated.get(), 6);
		assert_eq!(inner_proxies.borrow().len(), 3);
		assert_eq!(&**get_values.borrow(), [4, 2, 3]);
		assert_eq!(get_derived_values(), [6, 4, 5]);

		// Updating through the proxies must only update the single derived and other dependencies.
		inner_proxies.borrow()[0].set(7);
		assert_eq!(outer_derived_updated.get(), 3);
		assert_eq!(inner_derived_updated.get(), 7);
		assert_eq!(inner_proxies.borrow().len(), 3);
		assert_eq!(&**get_values.borrow(), [7, 2, 3]);
		assert_eq!(get_derived_values(), [9, 4, 5]);

		// Adding a new element should only create a single output
		signal.borrow_mut().push(0);
		assert_eq!(outer_derived_updated.get(), 4);
		assert_eq!(inner_derived_updated.get(), 11);
		assert_eq!(inner_proxies.borrow().len(), 4);
		assert_eq!(&**get_values.borrow(), [7, 2, 3, 0]);
		assert_eq!(get_derived_values(), [9, 4, 5, 2]);

		// Adding a new element should only update 3 outputs
		signal.borrow_mut().pop().expect("Should exist");
		assert_eq!(outer_derived_updated.get(), 5);
		assert_eq!(inner_derived_updated.get(), 14);
		assert_eq!(inner_proxies.borrow().len(), 4); // Note: We don't auto-remove the proxies
		assert_eq!(&**get_values.borrow(), [7, 2, 3]);
		assert_eq!(get_derived_values(), [9, 4, 5]);

		// Updating both the original and a proxy should update everything once, regardless of order
		{
			let dummy_trigger = Trigger::new();
			let _exec = dummy_trigger.exec();

			signal.borrow_mut()[0] = 1;
			inner_proxies.borrow()[1].set(3);
		}
		assert_eq!(outer_derived_updated.get(), 6);
		assert_eq!(inner_derived_updated.get(), 17);
		assert_eq!(inner_proxies.borrow().len(), 4);
		assert_eq!(&**get_values.borrow(), [1, 3, 3]);
		assert_eq!(get_derived_values(), [3, 5, 5]);
		{
			let dummy_trigger = Trigger::new();
			let _exec = dummy_trigger.exec();

			inner_proxies.borrow()[1].set(5);
			signal.borrow_mut()[0] = 2;
		}
		assert_eq!(outer_derived_updated.get(), 7);
		assert_eq!(inner_derived_updated.get(), 20);
		assert_eq!(inner_proxies.borrow().len(), 4);
		assert_eq!(&**get_values.borrow(), [2, 5, 3]);
		assert_eq!(get_derived_values(), [4, 7, 5]);
	}

	#[test]
	fn hash_map() {
		let signal = Signal::new(hash_map! {
			1 => 1,
			2 => 2,
			4 => 3,
		});

		let outer_derived_updated = Rc::new(Cell::new(0_usize));

		#[cloned(signal, outer_derived_updated)]
		let get_values = Derived::new(move || {
			outer_derived_updated.update(|value| value + 1);
			signal
				.borrow()
				.iter()
				.map(|(&key, &value)| (key, value))
				.sorted()
				.collect::<Vec<_>>()
		});

		let inner_derived_updated = Rc::new(Cell::new(0_usize));
		let inner_proxies = Rc::new(RefCell::new(HashMap::new()));

		#[cloned(inner_derived_updated, inner_proxies)]
		let split_signal = SplitSignal::new(signal.clone(), move |proxy| {
			inner_proxies.borrow_mut().insert(*proxy.key(), proxy.clone());

			#[cloned(inner_derived_updated)]
			Derived::new(move || {
				inner_derived_updated.update(|value| value + 1);
				proxy.get() + 2
			})
			.unsize()
		});


		let get_derived_values = move || {
			split_signal
				.borrow()
				.iter()
				.map(|(key, derived)| (key, derived.get()))
				.sorted()
				.collect::<Vec<_>>()
		};

		assert_eq!(outer_derived_updated.get(), 1);
		assert_eq!(inner_derived_updated.get(), 3);
		assert_eq!(inner_proxies.borrow().len(), 3);
		assert_eq!(&**get_values.borrow(), [(1, 1), (2, 2), (4, 3)]);
		assert_eq!(get_derived_values(), [(1, 3), (2, 4), (4, 5)]);

		// Updating through the original signal must update all derived without
		// re-creating them.
		signal.borrow_mut().insert(1, 4);
		assert_eq!(outer_derived_updated.get(), 2);
		assert_eq!(inner_derived_updated.get(), 6);
		assert_eq!(inner_proxies.borrow().len(), 3);
		assert_eq!(&**get_values.borrow(), [(1, 4), (2, 2), (4, 3)]);
		assert_eq!(get_derived_values(), [(1, 6), (2, 4), (4, 5)]);

		// Updating through the proxies must only update the single derived and other dependencies.
		inner_proxies.borrow()[&1].set(7);
		assert_eq!(outer_derived_updated.get(), 3);
		assert_eq!(inner_derived_updated.get(), 7);
		assert_eq!(inner_proxies.borrow().len(), 3);
		assert_eq!(&**get_values.borrow(), [(1, 7), (2, 2), (4, 3)]);
		assert_eq!(get_derived_values(), [(1, 9), (2, 4), (4, 5)]);

		// Adding a new element should only create a single output
		signal.borrow_mut().insert(6, 0);
		assert_eq!(outer_derived_updated.get(), 4);
		assert_eq!(inner_derived_updated.get(), 11);
		assert_eq!(inner_proxies.borrow().len(), 4);
		assert_eq!(&**get_values.borrow(), [(1, 7), (2, 2), (4, 3), (6, 0)]);
		assert_eq!(get_derived_values(), [(1, 9), (2, 4), (4, 5), (6, 2)]);

		// Adding a new element should only update 3 outputs
		signal.borrow_mut().remove(&6).expect("Should exist");
		assert_eq!(outer_derived_updated.get(), 5);
		assert_eq!(inner_derived_updated.get(), 14);
		assert_eq!(inner_proxies.borrow().len(), 4); // Note: We don't auto-remove the proxies
		assert_eq!(&**get_values.borrow(), [(1, 7), (2, 2), (4, 3)]);
		assert_eq!(get_derived_values(), [(1, 9), (2, 4), (4, 5)]);

		// Updating both the original and a proxy should update everything once, regardless of order
		{
			let dummy_trigger = Trigger::new();
			let _exec = dummy_trigger.exec();

			signal.borrow_mut().insert(1, 1);
			inner_proxies.borrow()[&2].set(3);
		}
		assert_eq!(outer_derived_updated.get(), 6);
		assert_eq!(inner_derived_updated.get(), 17);
		assert_eq!(inner_proxies.borrow().len(), 4);
		assert_eq!(&**get_values.borrow(), [(1, 1), (2, 3), (4, 3)]);
		assert_eq!(get_derived_values(), [(1, 3), (2, 5), (4, 5)]);
		{
			let dummy_trigger = Trigger::new();
			let _exec = dummy_trigger.exec();

			inner_proxies.borrow()[&2].set(5);
			signal.borrow_mut().insert(1, 2);
		}
		assert_eq!(outer_derived_updated.get(), 7);
		assert_eq!(inner_derived_updated.get(), 20);
		assert_eq!(inner_proxies.borrow().len(), 4);
		assert_eq!(&**get_values.borrow(), [(1, 2), (2, 5), (4, 3)]);
		assert_eq!(get_derived_values(), [(1, 4), (2, 7), (4, 5)]);
	}
}
