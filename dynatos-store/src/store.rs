//! Value store

// Imports
use {
	crate::handle::Handle,
	core::{
		any::{self, Any, TypeId},
		hash::BuildHasherDefault,
		marker::PhantomData,
	},
	dynatos_sync_types::{IMutRw, SyncBounds},
	dynatos_util::HoleyStack,
	std::{collections::HashMap, hash::DefaultHasher},
};

/// Value store
#[derive(Debug)]
pub struct ValueStore {
	values: ValuesImpl,
}

type ValuesImpl = IMutRw<HashMap<TypeId, StackImpl, BuildHasherDefault<DefaultHasher>>>;
type StackImpl = HoleyStack<Box<dyn Any + SyncBounds>>;

impl ValueStore {
	#[must_use]
	pub const fn new() -> Self {
		Self {
			values: IMutRw::new(HashMap::with_hasher(BuildHasherDefault::new())),
		}
	}

	/// Sets a value of `T` on this store.
	///
	/// This is equivalent to `push(value).forget()`
	pub fn set<T>(&self, value: T)
	where
		T: Any + SyncBounds,
	{
		self.push(value).forget();
	}

	/// Pushes a value of `T` to this store
	pub fn push<T>(&self, value: T) -> Handle<'_, T>
	where
		T: Any + SyncBounds,
	{
		let mut values = self.values.write();
		let stack = values.entry(TypeId::of::<T>()).or_default();
		let idx = stack.push(Box::new(value));

		Handle {
			store: self,
			idx,
			_phantom: PhantomData,
		}
	}

	/// Gets a value of `T` from this store
	#[must_use]
	pub fn try_get<T>(&self) -> Option<T>
	where
		T: Clone + 'static,
	{
		#[expect(
			clippy::redundant_closure_for_method_calls,
			reason = "Can't use `Option::cloned` due to inference issues"
		)]
		self.with::<T, _, _>(|value| value.clone())
	}

	/// Gets a value of `T` from this store.
	///
	/// # Panics
	/// Panics if the value does not exist
	#[must_use]
	#[track_caller]
	pub fn get<T>(&self) -> T
	where
		T: Clone + 'static,
	{
		self.try_get::<T>().unwrap_or_else(self::on_missing_value::<T, _>)
	}

	/// Uses a value of `T` from this store
	pub fn with<T, F, O>(&self, f: F) -> Option<O>
	where
		T: 'static,
		F: FnOnce(&T) -> O,
	{
		self.try_with(|value| value.map(f))
	}

	/// Uses a value of `T` from this store
	pub fn try_with<T, F, O>(&self, f: F) -> O
	where
		T: 'static,
		F: FnOnce(Option<&T>) -> O,
	{
		let values = self.values.read();
		let value = try {
			let stack = values.get(&TypeId::of::<T>())?;
			let value = stack.top()?;
			value.downcast_ref::<T>().expect("Value was the wrong type")
		};

		f(value)
	}

	/// Uses a value of `T` from this store, expecting it.
	#[track_caller]
	pub fn with_expect<T, F, O>(&self, f: F) -> O
	where
		T: 'static,
		F: FnOnce(&T) -> O,
	{
		self.with::<T, _, _>(f).unwrap_or_else(self::on_missing_value::<T, _>)
	}

	pub(crate) fn with_idx<F, O>(&self, idx: usize, type_id: TypeId, f: F) -> O
	where
		F: FnOnce(&(dyn Any + SyncBounds)) -> O,
	{
		let values = self.values.read();
		let stack = values.get(&type_id).expect("Value stack should exist");
		let value = stack.get(idx).expect("Value was already taken");
		f(&**value)
	}

	pub(crate) fn take_idx(&self, idx: usize, type_id: TypeId) -> Box<dyn Any + SyncBounds> {
		let mut values = self.values.write();
		let stack = values.get_mut(&type_id).expect("Value stack should exist");

		stack.pop(idx).expect("Value was already taken")
	}
}

impl Default for ValueStore {
	fn default() -> Self {
		Self::new()
	}
}

/// Called when value for type `T` was missing.
#[cold]
#[inline(never)]
#[track_caller]
fn on_missing_value<T, O>() -> O {
	panic!("Value for type {:?} was missing", any::type_name::<T>())
}
