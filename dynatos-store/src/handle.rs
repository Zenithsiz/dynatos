//! Handle

// Imports
use {
	crate::ValueStore,
	core::{
		any::{Any, TypeId},
		marker::PhantomData,
		mem,
	},
	dynatos_sync_types::SyncBounds,
};

/// A handle to a store value.
///
/// When dropped, the store value is also dropped.
#[must_use = "The handle object keeps a value in the store. If dropped, the value is also dropped"]
pub struct Handle<'a, T: 'static> {
	pub(crate) store:    &'a ValueStore,
	pub(crate) idx:      usize,
	pub(crate) _phantom: PhantomData<T>,
}

impl<'a, T: 'static> Handle<'a, T> {
	/// Converts this handle to an opaque handle
	pub const fn into_opaque(self) -> OpaqueHandle<'a> {
		// Create the opaque handle and forget ourselves
		// Note: This is to ensure we don't try to take the value in the [`Drop`] impl
		let handle = OpaqueHandle {
			store:   self.store,
			type_id: TypeId::of::<T>(),
			idx:     self.idx,
		};
		mem::forget(self);

		handle
	}

	/// Gets the value from this handle
	#[must_use]
	pub fn get(&self) -> T
	where
		T: Clone,
	{
		self.with(T::clone)
	}

	/// Uses the value from this handle
	pub fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&T) -> O,
	{
		self.store.with_idx(self.idx, TypeId::of::<T>(), |value| {
			let value = value.downcast_ref::<T>().expect("Value was the wrong type");
			f(value)
		})
	}

	/// Takes the value in this handle
	#[must_use = "If you only wish to drop the value, consider dropping the handle"]
	pub fn take(self) -> T {
		// Get the value and forget ourselves
		// Note: This is to ensure we don't try to take the value in the [`Drop`] impl
		let value = self.take_inner();
		mem::forget(self);

		value
	}

	/// Inner method for [`take`](Self::take), and the [`Drop`] impl.
	fn take_inner(&self) -> T {
		let value = self.store.take_idx(self.idx, TypeId::of::<T>());
		let value = value.downcast().expect("Value was the wrong type");

		*value
	}
}

impl<T: 'static> Drop for Handle<'_, T> {
	#[track_caller]
	fn drop(&mut self) {
		let _: T = self.take_inner();
	}
}

/// An opaque handle to a store value.
///
/// When dropped, the store value is also dropped.
#[must_use = "The handle object keeps a value in the store. If dropped, the value is also dropped"]
pub struct OpaqueHandle<'a> {
	pub(crate) store:   &'a ValueStore,
	pub(crate) type_id: TypeId,
	pub(crate) idx:     usize,
}

impl OpaqueHandle<'_> {
	/// Uses the value from this handle
	pub fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&(dyn Any + SyncBounds)) -> O,
	{
		self.store.with_idx(self.idx, self.type_id, f)
	}

	/// Takes the value in this handle
	#[must_use = "If you only wish to drop the value, consider dropping the handle"]
	pub fn take(self) -> Box<dyn Any> {
		// Get the value and forget ourselves
		// Note: This is to ensure we don't try to take the value in the [`Drop`] impl
		let value = self.take_inner();
		mem::forget(self);

		value
	}

	/// Inner method for [`take`](Self::take), and the [`Drop`] impl.
	fn take_inner(&self) -> Box<dyn Any + SyncBounds> {
		self.store.take_idx(self.idx, self.type_id)
	}
}

impl Drop for OpaqueHandle<'_> {
	#[track_caller]
	fn drop(&mut self) {
		let _: Box<dyn Any> = self.take_inner();
	}
}
