//! Context passing for `dynatos`

// Features
#![feature(try_blocks, thread_local, test, negative_impls, decl_macro, unsize)]

// Modules
pub mod world;

// Exports
pub use self::world::ContextWorld;

// Imports
use {
	self::world::{ContextStack, ContextStackOpaque},
	core::{any, marker::Unsize, mem},
	dynatos_world::WorldDefault,
};

/// A handle to a context value.
///
/// When dropped, the context value is also dropped.
#[must_use = "The handle object keeps a value in context. If dropped, the context is also dropped"]
pub struct Handle<T: 'static, W: ContextWorld = WorldDefault> {
	/// Handle
	handle: world::Handle<T, W>,
}

impl<T: 'static, W: ContextWorld> Handle<T, W> {
	/// Converts this handle to an opaque handle
	pub fn into_opaque(self) -> OpaqueHandle<W>
	where
		W::ContextStack<T>: ContextStackOpaque<W>,
	{
		// Create the opaque handle and forget ourselves
		// Note: This is to ensure we don't try to take the value in the [`Drop`] impl
		let handle = OpaqueHandle {
			handle: W::ContextStack::<T>::to_opaque(self.handle),
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
		W::ContextStack::<T>::with::<_, O>(self.handle, f)
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
		W::ContextStack::<T>::take(self.handle)
	}
}

impl<T: 'static, W: ContextWorld> Drop for Handle<T, W> {
	#[track_caller]
	fn drop(&mut self) {
		let _: T = self.take_inner();
	}
}

/// An opaque handle to a context value.
///
/// When dropped, the context value is also dropped.
#[must_use = "The handle object keeps a value in context. If dropped, the context is also dropped"]
pub struct OpaqueHandle<W: ContextWorld = WorldDefault> {
	/// Handle
	handle: world::OpaqueHandle<W>,
}

impl<W: ContextWorld> OpaqueHandle<W> {
	/// Uses the value from this handle
	pub fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&world::Any<W>) -> O,
	{
		W::ContextStackOpaque::with_opaque(self.handle, f)
	}

	/// Takes the value this handle is providing a context for.
	#[must_use = "If you only wish to drop the context, consider dropping the handle"]
	pub fn take(self) -> Box<world::Any<W>> {
		// Get the value and forget ourselves
		// Note: This is to ensure we don't try to take the value in the [`Drop`] impl
		let value = self.take_inner();
		mem::forget(self);

		value
	}

	/// Inner method for [`take`](Self::take), and the [`Drop`] impl.
	fn take_inner(&self) -> Box<world::Any<W>> {
		W::ContextStackOpaque::take_opaque(self.handle)
	}
}

impl<W: ContextWorld> Drop for OpaqueHandle<W> {
	#[track_caller]
	fn drop(&mut self) {
		let _: Box<world::Any<W>> = self.take_inner();
	}
}

/// Provides a value of `T` to the current context.
pub fn provide<T>(value: T) -> Handle<T> {
	self::provide_in::<T, WorldDefault>(value)
}

/// Provides a value of `T` to the current context in world `W`.
pub fn provide_in<T, W>(value: T) -> Handle<T, W>
where
	T: Unsize<world::Bounds<T, W>>,
	W: ContextWorld,
{
	// Push the value onto the stack
	let handle = W::ContextStack::<T>::push(value);

	Handle { handle }
}

/// Gets a value of `T` on the current context.
#[must_use]
pub fn get<T>() -> Option<T>
where
	T: Copy + 'static,
{
	self::get_in::<T, WorldDefault>()
}

/// Gets a value of `T` on the current context in world `W`.
#[must_use]
pub fn get_in<T, W>() -> Option<T>
where
	T: Copy + 'static,
	W: ContextWorld,
{
	#[expect(
		clippy::redundant_closure_for_method_calls,
		reason = "Can't use `Option::copied` due to inference issues"
	)]
	self::with_in::<T, _, _, W>(|value| value.copied())
}

/// Expects a value of `T` on the current context.
#[must_use]
#[track_caller]
pub fn expect<T>() -> T
where
	T: Copy + 'static,
{
	self::expect_in::<T, WorldDefault>()
}

/// Expects a value of `T` on the current context in world `W`.
#[must_use]
#[track_caller]
pub fn expect_in<T, W>() -> T
where
	T: Copy + 'static,
	W: ContextWorld,
{
	self::with_in::<T, _, _, W>(|value| *value.unwrap_or_else(self::on_missing_context::<T, _>))
}

/// Gets a cloned value of `T` on the current context.
#[must_use]
pub fn get_cloned<T>() -> Option<T>
where
	T: Clone + 'static,
{
	self::get_cloned_in::<T, WorldDefault>()
}

/// Gets a cloned value of `T` on the current context.
#[must_use]
pub fn get_cloned_in<T, W>() -> Option<T>
where
	T: Clone + 'static,
	W: ContextWorld,
{
	#[expect(
		clippy::redundant_closure_for_method_calls,
		reason = "Can't use `Option::cloned` due to inference issues"
	)]
	self::with_in::<T, _, _, W>(|value| value.cloned())
}

/// Expects a cloned value of `T` on the current context.
#[must_use]
#[track_caller]
pub fn expect_cloned<T>() -> T
where
	T: Clone + 'static,
{
	self::expect_cloned_in::<T, WorldDefault>()
}

/// Expects a cloned value of `T` on the current context in world `W`.
#[must_use]
#[track_caller]
pub fn expect_cloned_in<T, W>() -> T
where
	T: Clone + 'static,
	W: ContextWorld,
{
	self::with_in::<T, _, _, W>(|value| value.unwrap_or_else(self::on_missing_context::<T, _>).clone())
}

/// Uses a value of `T` on the current context.
pub fn with<T, F, O>(f: F) -> O
where
	T: 'static,
	F: FnOnce(Option<&T>) -> O,
{
	self::with_in::<T, F, O, WorldDefault>(f)
}

/// Uses a value of `T` on the current context in world `W`.
pub fn with_in<T, F, O, W>(f: F) -> O
where
	T: 'static,
	F: FnOnce(Option<&T>) -> O,
	W: ContextWorld,
{
	W::ContextStack::<T>::with_top(f)
}

/// Uses a value of `T` on the current context, expecting it.
#[track_caller]
pub fn with_expect<T, F, O>(f: F) -> O
where
	T: 'static,
	F: FnOnce(&T) -> O,
{
	self::with_expect_in::<T, F, O, WorldDefault>(f)
}

/// Uses a value of `T` on the current context, expecting it in world `W`.
#[track_caller]
pub fn with_expect_in<T, F, O, W>(f: F) -> O
where
	T: 'static,
	F: FnOnce(&T) -> O,
	W: ContextWorld,
{
	self::with_in::<T, _, _, W>(|value| value.map(f)).unwrap_or_else(self::on_missing_context::<T, _>)
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
		let handle = crate::provide::<usize>(5);

		assert_eq!(crate::get::<usize>(), Some(5));
		assert_eq!(handle.take(), 5);
		assert_eq!(crate::get::<usize>(), None);
	}

	#[test]
	fn stacked() {
		let handle1 = crate::provide::<usize>(5);
		let handle2 = crate::provide::<usize>(4);

		assert_eq!(crate::get::<usize>(), Some(4));
		assert_eq!(handle2.take(), 4);
		assert_eq!(crate::get::<usize>(), Some(5));
		assert_eq!(handle1.take(), 5);
		assert_eq!(crate::get::<usize>(), None);
	}

	#[test]
	fn stacked_swapped() {
		let handle1 = crate::provide::<usize>(5);
		let handle2 = crate::provide::<usize>(4);

		assert_eq!(crate::get::<usize>(), Some(4));
		assert_eq!(handle1.take(), 5);
		assert_eq!(crate::get::<usize>(), Some(4));
		assert_eq!(handle2.take(), 4);
		assert_eq!(crate::get::<usize>(), None);
	}

	#[test]
	fn stacked_triple() {
		let handle1 = crate::provide::<usize>(5);
		let handle2 = crate::provide::<usize>(4);
		let handle3 = crate::provide::<usize>(3);

		assert_eq!(crate::get::<usize>(), Some(3));
		assert_eq!(handle2.take(), 4);
		assert_eq!(handle3.take(), 3);
		assert_eq!(crate::get::<usize>(), Some(5));
		assert_eq!(handle1.take(), 5);
		assert_eq!(crate::get::<usize>(), None);
	}

	#[test]
	fn opaque() {
		let handle1 = crate::provide::<usize>(5).into_opaque();
		let handle2 = crate::provide::<usize>(4).into_opaque();

		assert_eq!(crate::get::<usize>(), Some(4));
		assert_eq!(*handle2.take().downcast::<usize>().expect("Handle had wrong type"), 4);
		assert_eq!(crate::get::<usize>(), Some(5));
		assert_eq!(*handle1.take().downcast::<usize>().expect("Handle had wrong type"), 5);
		assert_eq!(crate::get::<usize>(), None);
	}

	#[test]
	fn stress() {
		let handles_len = 100;
		let mut handles = (0..handles_len).map(crate::provide::<usize>).collect::<Vec<_>>();

		for value in (0..handles_len).rev() {
			assert_eq!(crate::get::<usize>(), Some(value));

			let handle = handles.pop().expect("Should have handle");
			assert_eq!(handle.get(), value);
			assert_eq!(handle.take(), value);
		}
		assert_eq!(crate::get::<usize>(), None);
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
		});
	}

	#[bench]
	fn access(bencher: &mut Bencher) {
		let _handle = crate::provide::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				let value = crate::get::<AccessTy>();
				test::black_box(value);
			}
		});
	}

	#[bench]
	fn access_expect(bencher: &mut Bencher) {
		let _handle = crate::provide::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				let value = crate::expect::<AccessTy>();
				test::black_box(value);
			}
		});
	}

	#[bench]
	fn access_with(bencher: &mut Bencher) {
		let _handle = crate::provide::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				crate::with::<AccessTy, _, _>(|value| test::black_box(value.copied()));
			}
		});
	}

	#[bench]
	fn access_with_expect(bencher: &mut Bencher) {
		let _handle = crate::provide::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				crate::with_expect::<AccessTy, _, _>(|value| test::black_box(*value));
			}
		});
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
		});
	}
}
