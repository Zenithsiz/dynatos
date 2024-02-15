//! Context passing for [`dynatos`]

// Features
#![feature(try_blocks)]

// Imports
use std::{
	any::{self, Any, TypeId},
	cell::RefCell,
	collections::HashMap,
	marker::PhantomData,
	mem,
};

type CtxsStack = RefCell<HashMap<TypeId, CtxStack>>;
type CtxStack = Vec<Option<Box<dyn Any>>>;

thread_local! {
	/// Context stack
	// TODO: Use type with less indirections?
	static CTXS_STACK: CtxsStack = RefCell::new(HashMap::new());
}

/// Uses the context stack for `T`
fn with_ctx_stack<T, F, O>(f: F) -> O
where
	T: 'static,
	F: FnOnce(Option<&CtxStack>) -> O,
{
	let type_id = TypeId::of::<T>();
	CTXS_STACK.with(|ctxs| {
		let ctxs = ctxs.try_borrow().expect("Cannot access context while modifying it");
		let stack = ctxs.get(&type_id);
		f(stack)
	})
}

/// Uses the context stack for `T` mutably
fn with_ctx_stack_mut<T, F, O>(f: F) -> O
where
	T: 'static,
	F: FnOnce(&mut CtxStack) -> O,
{
	let type_id = TypeId::of::<T>();
	CTXS_STACK.with(|ctxs| {
		let mut ctxs = ctxs.try_borrow_mut().expect("Cannot modify context while accessing it");
		let stack = ctxs.entry(type_id).or_default();
		f(stack)
	})
}

/// A handle to a context value.
///
/// When dropped, the context value is also dropped.
pub struct Handle<T: 'static> {
	/// Index
	value_idx: usize,

	/// Phantom
	// TODO: Variance?
	_phantom: PhantomData<T>,
}

impl<T: 'static> Handle<T> {
	/// Gets the value from this handle
	pub fn get(&self) -> T
	where
		T: Copy,
	{
		self.with(|value| *value)
	}

	/// Uses the value from this handle
	pub fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&T) -> O,
	{
		self::with_ctx_stack::<T, _, O>(|stack| {
			let value = stack
				.expect("Context stack should exist")
				.get(self.value_idx)
				.expect("Value was already taken")
				.as_ref()
				.expect("Value was already taken")
				.downcast_ref()
				.expect("Value was the wrong type");
			f(value)
		})
	}

	/// Takes the value this handle is providing a context for.
	pub fn take(self) -> T {
		// Get the value and forget ourselves
		// Note: This is to ensure we don't try to take the value in the [`Drop`] impl
		let value = self.take_inner();
		mem::forget(self);

		value
	}

	/// Inner method for [`take`](Self::take), and the [`Drop`] impl.
	fn take_inner(&self) -> T {
		self::with_ctx_stack_mut::<T, _, T>(|stack| {
			// Get the value
			let value = stack
				.get_mut(self.value_idx)
				.and_then(Option::take)
				.expect("Value was already taken")
				.downcast()
				.expect("Value was the wrong type");

			// Then remove any empty entries from the end
			while stack.last().is_some_and(|value| value.is_none()) {
				stack.pop().expect("Should have a value at the end");
			}

			*value
		})
	}
}

impl<T: 'static> Drop for Handle<T> {
	#[track_caller]
	fn drop(&mut self) {
		let _ = self.take_inner();
	}
}

/// Provides a value of `T` to the current context.
pub fn provide<T>(value: T) -> Handle<T>
where
	T: Any,
{
	// Push the value onto the stack
	self::with_ctx_stack_mut::<T, _, _>(|stack| {
		let value_idx = stack.len();
		stack.push(Some(Box::new(value)));

		Handle {
			value_idx,
			_phantom: PhantomData,
		}
	})
}

/// Gets a value of `T` on the current context.
pub fn get<T>() -> Option<T>
where
	T: 'static,
	T: Copy,
{
	self::with::<T, _, _>(|value| value.copied())
}

/// Expects a value of `T` on the current context.
#[track_caller]
pub fn expect<T>() -> T
where
	T: 'static,
	T: Copy,
{
	self::get::<T>().unwrap_or_else(self::on_missing_context::<T, _>)
}

/// Gets a cloned value of `T` on the current context.
pub fn get_cloned<T>() -> Option<T>
where
	T: 'static,
	T: Clone,
{
	self::with::<T, _, _>(|value| value.cloned())
}

/// Expects a cloned value of `T` on the current context.
#[track_caller]
pub fn expect_cloned<T>() -> T
where
	T: 'static,
	T: Clone,
{
	self::get_cloned::<T>().unwrap_or_else(self::on_missing_context::<T, _>)
}

/// Uses a value of `T` on the current context.
pub fn with<T, F, O>(f: F) -> O
where
	T: 'static,
	F: FnOnce(Option<&T>) -> O,
{
	self::with_ctx_stack::<T, _, _>(|stack| {
		let value = try {
			stack?
				.last()?
				.as_ref()
				.expect("Value was taken")
				.downcast_ref::<T>()
				.expect("Value was the wrong type")
		};
		f(value)
	})
}

/// Uses a value of `T` on the current context, expecting it.
#[track_caller]
pub fn with_expect<T, F, O>(f: F) -> O
where
	T: 'static,
	F: FnOnce(&T) -> O,
{
	self::with(|value| value.map(f)).unwrap_or_else(self::on_missing_context::<T, _>)
}

/// Called when context for type `T` was missing.
#[cold]
#[inline(never)]
#[track_caller]
fn on_missing_context<T, O>() -> O {
	panic!("Context for type {:?} was missing", any::type_name::<T>())
}

#[cfg(test)]
mod test {
	#[test]
	fn simple() {
		let handle = crate::provide::<i32>(5);

		assert_eq!(crate::get::<i32>(), Some(5));
		assert_eq!(handle.take(), 5);
		assert_eq!(crate::get::<i32>(), None);
	}

	#[test]
	fn stacked() {
		let handle1 = crate::provide::<i32>(5);
		let handle2 = crate::provide::<i32>(4);

		assert_eq!(crate::get::<i32>(), Some(4));
		assert_eq!(handle2.take(), 4);
		assert_eq!(crate::get::<i32>(), Some(5));
		assert_eq!(handle1.take(), 5);
		assert_eq!(crate::get::<i32>(), None);
	}

	#[test]
	fn stacked_swapped() {
		let handle1 = crate::provide::<i32>(5);
		let handle2 = crate::provide::<i32>(4);

		assert_eq!(crate::get::<i32>(), Some(4));
		assert_eq!(handle1.take(), 5);
		assert_eq!(crate::get::<i32>(), Some(4));
		assert_eq!(handle2.take(), 4);
		assert_eq!(crate::get::<i32>(), None);
	}

	#[test]
	fn stacked_triple() {
		let handle1 = crate::provide::<i32>(5);
		let handle2 = crate::provide::<i32>(4);
		let handle3 = crate::provide::<i32>(3);

		assert_eq!(crate::get::<i32>(), Some(3));
		assert_eq!(handle2.take(), 4);
		assert_eq!(handle3.take(), 3);
		assert_eq!(crate::get::<i32>(), Some(5));
		assert_eq!(handle1.take(), 5);
		assert_eq!(crate::get::<i32>(), None);
	}

	#[test]
	fn stress() {
		let handles_len = 100;
		let mut handles = (0..handles_len).map(crate::provide::<i32>).collect::<Vec<_>>();

		for value in (0..handles_len).rev() {
			assert_eq!(crate::get::<i32>(), Some(value));

			let handle = handles.pop().expect("Should have handle");
			assert_eq!(handle.get(), value);
			assert_eq!(handle.take(), value);
		}
		assert_eq!(crate::get::<i32>(), None);
	}
}
