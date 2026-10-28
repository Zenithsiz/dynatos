//! [`Vec`] impl of [`Collection`]

use {
	super::{Collection, CollectionMut, CollectionStorage, StorageElement},
	crate::Trigger,
	core::cell::RefCell,
};


impl<T> Collection for Vec<T> {
	type Key = usize;
	type Value = T;

	fn index(&self, key: &Self::Key) -> &Self::Value {
		&self[*key]
	}

	fn iter(&self) -> impl Iterator<Item = (Self::Key, &Self::Value)> {
		self.as_slice().iter().enumerate()
	}

	fn values(&self) -> impl Iterator<Item = &Self::Value> {
		self.as_slice().iter()
	}
}

impl<T> CollectionMut for Vec<T> {
	fn index_mut(&mut self, key: &Self::Key) -> &mut Self::Value {
		&mut self[*key]
	}
}

impl<T: 'static> CollectionStorage for Vec<T> {
	type Storage<O> = Vec<StorageElement<O>>;

	fn sync_storage<O>(&self, storage: &RefCell<Self::Storage<O>>, f: impl Fn(Self::Key, Trigger) -> O) {
		let mut storage_borrow = storage.borrow_mut();
		match self.len() <= storage_borrow.len() {
			true => storage_borrow.truncate(self.len()),
			false => {
				let start_idx = storage_borrow.len();
				drop(storage_borrow);

				for idx in start_idx..self.len() {
					let trigger = Trigger::new();
					let value = f(idx, trigger.clone());
					storage.borrow_mut().push(StorageElement::new(value, trigger));
				}
			},
		}
	}
}


#[cfg(test)]
mod tests {
	use {
		crate::{Derived, Signal, SignalBorrow, SignalBorrowMut, SignalGet, SignalSet, SplitSignal, Trigger},
		core::cell::{Cell, RefCell},
		std::rc::Rc,
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
}
