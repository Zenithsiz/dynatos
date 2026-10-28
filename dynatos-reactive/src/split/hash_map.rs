//! Hash map signal split

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
	std::{
		collections::{HashMap, hash_map},
		rc::Rc,
	},
	zutil_cloned::cloned,
};

#[derive(Debug)]
struct Output<O> {
	value:   Option<O>,
	trigger: Trigger,
}

#[derive(Debug)]
struct Inner<K, O> {
	output:         RefCell<HashMap<K, Output<O>>>,
	update_proxies: Effect,
}

/// Hash map signal split
///
/// Allows splitting a signal with a `HashMap` value into
/// a `HashMap` of signals.
#[derive(Debug)]
pub struct HashMapSplitSignal<K, O> {
	inner: Rc<Inner<K, O>>,
}

impl<K, O> HashMapSplitSignal<K, O> {
	pub fn new<S, F>(signal: S, f: F) -> Self
	where
		K: Eq + Hash + Clone + 'static,
		O: 'static,
		S: Clone + for<'a> SignalBorrow<Ref<'a>: Deref<Target: HashMapSplit<Key = K>>> + 'static,
		F: Fn(HashMapSplitSignalProxy<S, K, O>) -> O + 'static,
	{
		let inner = Rc::<Inner<K, O>>::new_cyclic(move |inner| {
			#[cloned(inner)]
			let update_proxies = Effect::new_raw(move || {
				let Some(inner) = inner.upgrade() else { return };

				{
					let output = inner.output.borrow();
					#[expect(clippy::iter_over_hash_type, reason = "Order doesn't matter")]
					for value in output.as_map().values() {
						// TODO: Should we drop the output when calling this?
						//       Maybe use a dummy trigger execution to delay this
						//       until after we drop the output?
						value.trigger.exec();
					}
				}

				let input = signal.borrow();
				let mut output = inner.output.borrow_mut();

				// TODO: Not have to go over both maps to add/remove?
				output.as_map_mut().retain(|key, _| input.as_map().contains_key(key));
				#[expect(clippy::iter_over_hash_type, reason = "Order doesn't matter")]
				for key in input.as_map().keys() {
					let hash_map::Entry::Vacant(entry) = output.as_map_mut().entry(key.clone()) else {
						continue;
					};

					entry.insert(Output {
						value:   None,
						trigger: Trigger::new(),
					});
					drop(output);

					let proxy = HashMapSplitSignalProxy {
						inner:  inner.share(),
						signal: signal.clone(),
						key:    key.clone(),
					};
					let value = f(proxy);
					output = inner.output.borrow_mut();
					output.get_mut(key).expect("Should exist").value = Some(value);
				}
			});

			Inner {
				output: RefCell::new(HashMap::new()),
				update_proxies,
			}
		});

		// Note: Needs to run outside because the `Inner` isn't constructed yet.
		inner.update_proxies.run();

		Self { inner }
	}
}

impl<K, O> SignalBorrow for HashMapSplitSignal<K, O> {
	type Ref<'a>
		= HashMapSplitSignalBorrow<'a, K, O>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		HashMapSplitSignalBorrow(self.inner.output.borrow())
	}
}

#[derive(Debug)]
pub struct HashMapSplitSignalBorrow<'a, K, O>(cell::Ref<'a, HashMap<K, Output<O>>>);

impl<K, O> HashMapSplitSignalBorrow<'_, K, O> {
	pub fn iter(&self) -> impl Iterator<Item = (&K, &O)> {
		self.0.iter().map(|(key, output)| {
			let value = output.value.as_ref().expect("Value was missing");
			(key, value)
		})
	}
}

#[derive(Clone, Debug)]
pub struct HashMapSplitSignalProxy<S, K, O> {
	inner:  Rc<Inner<K, O>>,
	signal: S,
	key:    K,
}

impl<S, K, O> HashMapSplitSignalProxy<S, K, O> {
	/// Returns the key of this proxy
	pub const fn key(&self) -> &K {
		&self.key
	}
}

impl<S, K, O> SignalBorrow for HashMapSplitSignalProxy<S, K, O>
where
	S: SignalBorrow,
	K: Eq + Hash + Clone,
{
	type Ref<'a>
		= HashMapSplitSignalProxyBorrow<'a, S, K>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		self.inner.output.borrow()[&self.key].trigger.gather_subs();

		HashMapSplitSignalProxyBorrow {
			borrow: self.signal.borrow_no_dep(),
			key:    self.key.clone(),
		}
	}
}

impl<S, K, O> SignalBorrowMut for HashMapSplitSignalProxy<S, K, O>
where
	S: SignalBorrowMut,
	K: Eq + Hash + Clone,
{
	type RefMut<'a>
		= HashMapSplitSignalProxyBorrowMut<'a, S, K>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let exec = self.inner.output.borrow()[&self.key].trigger.exec();

		HashMapSplitSignalProxyBorrowMut {
			_suppress: self.inner.update_proxies.suppress(),
			borrow:    self.signal.borrow_mut(),
			_exec:     exec,
			key:       self.key.clone(),
		}
	}
}

impl<S, K, O> SignalGetDefaultImpl for HashMapSplitSignalProxy<S, K, O> {}
impl<S, K, O> SignalGetClonedDefaultImpl for HashMapSplitSignalProxy<S, K, O> {}
impl<S, K, O> SignalWithDefaultImpl for HashMapSplitSignalProxy<S, K, O> {}

impl<S, K, O> SignalSetDefaultImpl for HashMapSplitSignalProxy<S, K, O> {}
impl<S, K, O> SignalUpdateDefaultImpl for HashMapSplitSignalProxy<S, K, O> {}

#[derive(Debug)]
pub struct HashMapSplitSignalProxyBorrow<'a, S, K>
where
	S: SignalBorrow + 'a,
{
	borrow: S::Ref<'a>,
	key:    K,
}

impl<'a, S, K> ops::Deref for HashMapSplitSignalProxyBorrow<'a, S, K>
where
	S: SignalBorrow<Ref<'a>: Deref<Target: HashMapSplit<Key = K>>>,
	K: Eq + Hash,
{
	type Target = <<S::Ref<'a> as Deref>::Target as HashMapSplit>::Value;

	fn deref(&self) -> &Self::Target {
		&self.borrow.as_map()[&self.key]
	}
}

#[derive(Debug)]
pub struct HashMapSplitSignalProxyBorrowMut<'a, S, K>
where
	S: SignalBorrowMut + 'a,
{
	_suppress: EffectSuppressed<'a, dyn EffectRun>,
	borrow:    S::RefMut<'a>,
	_exec:     Option<TriggerExec>,
	key:       K,
}

impl<'a, S, K> ops::Deref for HashMapSplitSignalProxyBorrowMut<'a, S, K>
where
	S: SignalBorrowMut<RefMut<'a>: Deref<Target: HashMapSplit<Key = K>>>,
	K: Eq + Hash,
{
	type Target = <<S::RefMut<'a> as Deref>::Target as HashMapSplit>::Value;

	fn deref(&self) -> &Self::Target {
		&self.borrow.as_map()[&self.key]
	}
}

impl<'a, S, K> ops::DerefMut for HashMapSplitSignalProxyBorrowMut<'a, S, K>
where
	S: SignalBorrowMut<RefMut<'a>: DerefMut<Target: HashMapSplitMut<Key = K>>>,
	K: Eq + Hash,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.borrow.as_map_mut().get_mut(&self.key).expect("Value was missing")
	}
}

pub trait HashMapSplit {
	type Key: 'static;
	type Value: 'static;

	fn as_map(&self) -> &HashMap<Self::Key, Self::Value>;
}

pub trait HashMapSplitMut: HashMapSplit {
	fn as_map_mut(&mut self) -> &mut HashMap<Self::Key, Self::Value>;
}

impl<V> HashMapSplit for &'_ V
where
	V: HashMapSplit,
{
	type Key = V::Key;
	type Value = V::Value;

	fn as_map(&self) -> &HashMap<Self::Key, Self::Value> {
		(**self).as_map()
	}
}

impl<V> HashMapSplit for &'_ mut V
where
	V: HashMapSplit,
{
	type Key = V::Key;
	type Value = V::Value;

	fn as_map(&self) -> &HashMap<Self::Key, Self::Value> {
		(**self).as_map()
	}
}

impl<V> HashMapSplitMut for &'_ mut V
where
	V: HashMapSplitMut,
{
	fn as_map_mut(&mut self) -> &mut HashMap<Self::Key, Self::Value> {
		(**self).as_map_mut()
	}
}

#[expect(clippy::implicit_hasher, reason = "Would require changing the trait")]
impl<K: 'static, V: 'static> HashMapSplit for HashMap<K, V> {
	type Key = K;
	type Value = V;

	fn as_map(&self) -> &HashMap<Self::Key, Self::Value> {
		self
	}
}

#[expect(clippy::implicit_hasher, reason = "Would require changing the trait")]
impl<K: 'static, V: 'static> HashMapSplitMut for HashMap<K, V> {
	fn as_map_mut(&mut self) -> &mut HashMap<Self::Key, Self::Value> {
		self
	}
}

#[cfg(test)]
mod tests {
	use {
		super::*,
		crate::{Derived, Signal, SignalBorrowMut, SignalGet, SignalSet},
		core::cell::Cell,
		itertools::Itertools,
		std::hash_map,
	};


	#[test]
	fn basic() {
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
		let split_signal = HashMapSplitSignal::new(signal.clone(), move |proxy| {
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
				.map(|(&key, derived)| (key, derived.get()))
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
