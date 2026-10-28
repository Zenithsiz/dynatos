//! [`HashMap`] impl of [`Split`]

use {
	super::{Split, SplitMut, SplitStorage, StorageElement},
	crate::Trigger,
	core::{
		cell::RefCell,
		hash::{BuildHasher, Hash},
	},
	std::collections::HashMap,
};


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

	fn sync_storage<O>(&self, storage: &RefCell<Self::Storage<O>>, f: impl Fn(Self::Key, Trigger) -> O) {
		// TODO: Not have to go over both maps to add/remove?
		storage.borrow_mut().retain(|key, _| self.contains_key(key));
		#[expect(clippy::iter_over_hash_type, reason = "Order doesn't matter")]
		for key in self.keys() {
			if storage.borrow().contains_key(key) {
				continue;
			}

			let trigger = Trigger::new();
			let value = f(key.clone(), trigger.clone());
			storage
				.borrow_mut()
				.insert(key.clone(), StorageElement { value, trigger });
		}
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
