//! Context passing for `dynatos`

// Features
#![feature(
	try_blocks,
	thread_local,
	test,
	const_collections_with_hasher,
	negative_impls,
	decl_macro,
	lint_reasons
)]

// Imports
use std::{
	any::{self, Any, TypeId},
	cell::RefCell,
	collections::HashMap,
	hash::{BuildHasher, DefaultHasher},
	marker::PhantomData,
	mem,
};

type CtxsStack = RefCell<HashMap<TypeId, CtxStack, RandomState>>;
type CtxStack = Vec<Option<Box<dyn Any>>>;

/// Hash builder for `CTXS_STACK`
struct RandomState;

impl BuildHasher for RandomState {
	type Hasher = DefaultHasher;

	fn build_hasher(&self) -> Self::Hasher {
		DefaultHasher::default()
	}
}

/// Context stack
// TODO: Use type with less indirections?
#[thread_local]
static CTXS_STACK: CtxsStack = RefCell::new(HashMap::with_hasher(RandomState));

/// Uses the context stack for `T`
fn with_ctx_stack<T, F, O>(f: F) -> O
where
	T: 'static,
	F: FnOnce(Option<&CtxStack>) -> O,
{
	let type_id = TypeId::of::<T>();
	self::with_ctx_stack_opaque(type_id, f)
}

/// Uses the context stack for `T` with a type id
fn with_ctx_stack_opaque<F, O>(type_id: TypeId, f: F) -> O
where
	F: FnOnce(Option<&CtxStack>) -> O,
{
	let ctxs = CTXS_STACK
		.try_borrow()
		.expect("Cannot access context while modifying it");
	let stack = ctxs.get(&type_id);
	f(stack)
}

/// Uses the context stack for `T` mutably
fn with_ctx_stack_mut<T, F, O>(f: F) -> O
where
	T: 'static,
	F: FnOnce(&mut CtxStack) -> O,
{
	let type_id = TypeId::of::<T>();
	self::with_ctx_stack_mut_opaque(type_id, f)
}

/// Uses the context stack for `T` mutably with a type id
fn with_ctx_stack_mut_opaque<F, O>(type_id: TypeId, f: F) -> O
where
	F: FnOnce(&mut CtxStack) -> O,
{
	let mut ctxs = CTXS_STACK
		.try_borrow_mut()
		.expect("Cannot modify context while accessing it");
	let stack = ctxs.entry(type_id).or_default();
	f(stack)
}

/// A handle to a context value.
///
/// When dropped, the context value is also dropped.
#[must_use = "The handle object keeps a value in context. If dropped, the context is also dropped"]
pub struct Handle<T: 'static> {
	/// Index
	value_idx: usize,

	/// Phantom
	// TODO: Variance?
	_phantom: PhantomData<T>,
}

impl<T: 'static> Handle<T> {
	/// Converts this handle to an opaque handle
	pub fn into_opaque(self) -> OpaqueHandle {
		// Create the opaque handle and forget ourselves
		// Note: This is to ensure we don't try to take the value in the [`Drop`] impl
		let handle = OpaqueHandle {
			value_idx: self.value_idx,
			type_id:   TypeId::of::<T>(),
		};
		mem::forget(self);

		handle
	}

	/// Gets the value from this handle
	#[must_use]
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
	#[must_use = "If you only wish to drop the context, consider dropping the handle"]
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

impl<T> !Send for Handle<T> {}
impl<T> !Sync for Handle<T> {}

impl<T: 'static> Drop for Handle<T> {
	#[track_caller]
	fn drop(&mut self) {
		let _ = self.take_inner();
	}
}

/// An opaque handle to a context value.
///
/// When dropped, the context value is also dropped.
#[must_use = "The handle object keeps a value in context. If dropped, the context is also dropped"]
pub struct OpaqueHandle {
	/// Index
	value_idx: usize,

	/// Type id
	type_id: TypeId,
}

impl OpaqueHandle {
	/// Uses the value from this handle
	pub fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&dyn Any) -> O,
	{
		self::with_ctx_stack_opaque(self.type_id, |stack| {
			let value = stack
				.expect("Context stack should exist")
				.get(self.value_idx)
				.expect("Value was already taken")
				.as_ref()
				.expect("Value was already taken");
			f(value)
		})
	}

	/// Takes the value this handle is providing a context for.
	#[must_use = "If you only wish to drop the context, consider dropping the handle"]
	pub fn take(self) -> Box<dyn Any> {
		// Get the value and forget ourselves
		// Note: This is to ensure we don't try to take the value in the [`Drop`] impl
		let value = self.take_inner();
		mem::forget(self);

		value
	}

	/// Inner method for [`take`](Self::take), and the [`Drop`] impl.
	fn take_inner(&self) -> Box<dyn Any> {
		self::with_ctx_stack_mut_opaque(self.type_id, |stack| {
			// Get the value
			let value = stack
				.get_mut(self.value_idx)
				.and_then(Option::take)
				.expect("Value was already taken");

			// Then remove any empty entries from the end
			while stack.last().is_some_and(|value| value.is_none()) {
				stack.pop().expect("Should have a value at the end");
			}

			value
		})
	}
}

impl Drop for OpaqueHandle {
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
#[must_use]
pub fn get<T>() -> Option<T>
where
	T: 'static,
	T: Copy,
{
	self::with::<T, _, _>(|value| value.copied())
}

/// Expects a value of `T` on the current context.
#[must_use]
#[track_caller]
pub fn expect<T>() -> T
where
	T: 'static,
	T: Copy,
{
	self::with::<T, _, _>(|value| *value.unwrap_or_else(self::on_missing_context::<T, _>))
}

/// Gets a cloned value of `T` on the current context.
#[must_use]
pub fn get_cloned<T>() -> Option<T>
where
	T: 'static,
	T: Clone,
{
	self::with::<T, _, _>(|value| value.cloned())
}

/// Expects a cloned value of `T` on the current context.
#[must_use]
#[track_caller]
pub fn expect_cloned<T>() -> T
where
	T: 'static,
	T: Clone,
{
	self::with::<T, _, _>(|value| value.unwrap_or_else(self::on_missing_context::<T, _>).clone())
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
	// Imports
	extern crate test;
	use test::Bencher;

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

	// Type and value to test for the accesses
	type AccessTy = usize;
	const ACCESS_TY_DEFAULT: AccessTy = 123;

	// Number of times to run each iteration
	const REPEAT_COUNT: usize = 100;

	// Reference benchmark.
	#[bench]
	fn access_static(bencher: &mut Bencher) {
		static VALUE: AccessTy = ACCESS_TY_DEFAULT;

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				let value = VALUE;
				test::black_box(value);
			}
		})
	}

	#[bench]
	fn access(bencher: &mut Bencher) {
		let _handle = crate::provide::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				let value = crate::get::<AccessTy>();
				test::black_box(value);
			}
		})
	}

	#[bench]
	fn access_expect(bencher: &mut Bencher) {
		let _handle = crate::provide::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				let value = crate::expect::<AccessTy>();
				test::black_box(value);
			}
		})
	}

	#[bench]
	fn access_with(bencher: &mut Bencher) {
		let _handle = crate::provide::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				crate::with::<AccessTy, _, _>(|value| test::black_box(value.copied()));
			}
		})
	}

	#[bench]
	fn access_with_expect(bencher: &mut Bencher) {
		let _handle = crate::provide::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				crate::with_expect::<AccessTy, _, _>(|value| test::black_box(*value));
			}
		})
	}

	/// Creates several types and attempts to access them all.
	#[bench]
	fn access_many_types(bencher: &mut Bencher) {
		macro decl_provide_ty($($T:ident),* $(,)?) {
			$(
				#[derive(Clone, Copy)]
				#[expect(dead_code, reason = "Used only for benchmarking")]
				struct $T(usize);
				let _handle = crate::provide::<$T>( $T(0) );
			)*
		}

		decl_provide_ty! { T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49 }

		macro use_ty($($T:ident),* $(,)?) {
			$(
				crate::with_expect::<$T, _, _>(|value| test::black_box(*value));
			)*
		}

		bencher.iter(|| {
			use_ty! { T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49 }
		})
	}
}
