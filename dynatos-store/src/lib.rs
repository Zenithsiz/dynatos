//! Value store for `dynatos`

// Features
#![feature(try_blocks, decl_macro)]
#![cfg_attr(test, feature(test))]

// Modules
mod handle;
mod store;

// Exports
pub use self::store::ValueStore;

#[cfg(test)]
mod tests {
	// Imports
	extern crate test;
	use {super::*, test::Bencher};

	#[test]
	fn simple() {
		let store = ValueStore::new();

		let handle = store.push::<usize>(5);

		assert_eq!(store.get::<usize>(), Some(5));
		assert_eq!(handle.take(), 5);
		assert_eq!(store.get::<usize>(), None);
	}

	#[test]
	fn stacked() {
		let store = ValueStore::new();

		let handle1 = store.push::<usize>(5);
		let handle2 = store.push::<usize>(4);

		assert_eq!(store.get::<usize>(), Some(4));
		assert_eq!(handle2.take(), 4);
		assert_eq!(store.get::<usize>(), Some(5));
		assert_eq!(handle1.take(), 5);
		assert_eq!(store.get::<usize>(), None);
	}

	#[test]
	fn stacked_swapped() {
		let store = ValueStore::new();

		let handle1 = store.push::<usize>(5);
		let handle2 = store.push::<usize>(4);

		assert_eq!(store.get::<usize>(), Some(4));
		assert_eq!(handle1.take(), 5);
		assert_eq!(store.get::<usize>(), Some(4));
		assert_eq!(handle2.take(), 4);
		assert_eq!(store.get::<usize>(), None);
	}

	#[test]
	fn stacked_triple() {
		let store = ValueStore::new();

		let handle1 = store.push::<usize>(5);
		let handle2 = store.push::<usize>(4);
		let handle3 = store.push::<usize>(3);

		assert_eq!(store.get::<usize>(), Some(3));
		assert_eq!(handle2.take(), 4);
		assert_eq!(handle3.take(), 3);
		assert_eq!(store.get::<usize>(), Some(5));
		assert_eq!(handle1.take(), 5);
		assert_eq!(store.get::<usize>(), None);
	}

	#[test]
	fn opaque() {
		let store = ValueStore::new();

		let handle1 = store.push::<usize>(5).into_opaque();
		let handle2 = store.push::<usize>(4).into_opaque();

		assert_eq!(store.get::<usize>(), Some(4));
		assert_eq!(*handle2.take().downcast::<usize>().expect("Handle had wrong type"), 4);
		assert_eq!(store.get::<usize>(), Some(5));
		assert_eq!(*handle1.take().downcast::<usize>().expect("Handle had wrong type"), 5);
		assert_eq!(store.get::<usize>(), None);
	}

	#[test]
	fn stress() {
		let store = ValueStore::new();

		let handles_len = 100;
		let mut handles = (0..handles_len).map(|idx| store.push(idx)).collect::<Vec<_>>();

		for value in (0..handles_len).rev() {
			assert_eq!(store.get::<usize>(), Some(value));

			let handle = handles.pop().expect("Should have handle");
			assert_eq!(handle.get(), value);
			assert_eq!(handle.take(), value);
		}
		assert_eq!(store.get::<usize>(), None);
	}

	// Type and value to test for the accesses
	type AccessTy = usize;
	const ACCESS_TY_DEFAULT: AccessTy = 123;

	// Number of times to run each iteration
	const REPEAT_COUNT: usize = 100;

	// Reference benchmark.
	#[bench]
	fn access_local(bencher: &mut Bencher) {
		let value: AccessTy = ACCESS_TY_DEFAULT;

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				test::black_box(value);
			}
		});
	}

	#[bench]
	fn access_get(bencher: &mut Bencher) {
		let store = ValueStore::new();

		let _handle = store.push::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				let value = store.get::<AccessTy>();
				test::black_box(value);
			}
		});
	}

	#[bench]
	fn access_expect(bencher: &mut Bencher) {
		let store = ValueStore::new();

		let _handle = store.push::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				let value = store.expect::<AccessTy>();
				test::black_box(value);
			}
		});
	}

	#[bench]
	fn access_with(bencher: &mut Bencher) {
		let store = ValueStore::new();

		let _handle = store.push::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				store.with::<AccessTy, _, _>(|&value| test::black_box(value));
			}
		});
	}

	#[bench]
	fn access_with_expect(bencher: &mut Bencher) {
		let store = ValueStore::new();

		let _handle = store.push::<AccessTy>(ACCESS_TY_DEFAULT);

		bencher.iter(|| {
			for _ in 0..test::black_box(REPEAT_COUNT) {
				store.with_expect::<AccessTy, _, _>(|value| test::black_box(*value));
			}
		});
	}

	/// Creates several types and attempts to access them all.
	#[bench]
	fn access_many_types(bencher: &mut Bencher) {
		let store = ValueStore::new();

		macro decl_provide_ty($($T:ident),* $(,)?) {
			$(
				#[derive(Clone, Copy)]
				#[expect(dead_code, reason = "Used only for benchmarking")]
				struct $T(usize);
				let _handle = store.push::<$T>( $T(0) );
			)*
		}

		decl_provide_ty! { T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49 }

		macro use_ty($($T:ident),* $(,)?) {
			$(
				store.with_expect::<$T, _, _>(|value| test::black_box(*value));
			)*
		}

		bencher.iter(|| {
			use_ty! { T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49 }
		});
	}
}
