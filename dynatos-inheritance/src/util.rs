//! Utilities

// Imports
use core::{
	mem,
	ptr::{self, NonNull},
};

/// Marker trait for types that contain a type `T`
/// at offset 0.
///
/// # Safety
/// You must ensure that `Self` contains a type `T`
/// at offset 0, with suitable alignment.
pub const unsafe trait Contains<T>: Sized {
	/// Casts a `NonNull<Self>` to `NonNull<T>`
	#[must_use]
	fn to_non_null(ptr: NonNull<Self>) -> NonNull<T> {
		ptr.cast()
	}

	/// Casts a `NonNull<T>` to `NonNull<Self>`
	#[must_use]
	fn from_non_null(ptr: NonNull<T>) -> NonNull<Self> {
		ptr.cast()
	}
}

// SAFETY: `T` always contains itself at offset 0
unsafe impl<T> const Contains<T> for T {}

/// Marker trait for types that are `repr(transparent)`
/// over a single type `T`.
///
/// # Safety
/// You must ensure that `Self` is `repr(transparent)`
/// over a single type `T`.
pub const unsafe trait ReprTransparent<T>: Sized {
	/// Casts a `&T` to `&Self`
	#[must_use]
	fn to_ref(ref_: &Self) -> &T {
		// SAFETY: Implementor ensures that `Self` is `repr(transparent)`
		//         over a single `T` field.
		unsafe { ptr::from_ref(ref_).cast::<T>().as_ref_unchecked() }
	}

	/// Casts a `&Self` to `&T`
	///
	/// # Safety
	/// You must ensure that the value of `T` is valid for `Self`
	#[must_use]
	unsafe fn from_ref(ref_: &T) -> &Self {
		// SAFETY: Implementor ensures that `Self` is `repr(transparent)`
		//         over a single `T` field.
		unsafe { ptr::from_ref(ref_).cast::<Self>().as_ref_unchecked() }
	}

	/// Converts a `T` to `Self`
	///
	/// # Safety
	/// You must ensure that the value of `T` is valid for `Self`
	unsafe fn from_repr(value: T) -> Self {
		// SAFETY: Implementor ensures that `Self` is `repr(transparent)`
		//         over a single `T` field.
		//         Caller ensures that the value is valid for `Self`.
		let output = unsafe { mem::transmute_copy::<T, Self>(&value) };
		mem::forget(value);

		output
	}

	/// Converts a `Self` to `T`
	fn into_repr(self) -> T {
		// SAFETY: Implementor ensures that `Self` is `repr(transparent)`
		//         over a single `T` field.
		let value = unsafe { mem::transmute_copy::<Self, T>(&self) };
		mem::forget(self);

		value
	}
}

// SAFETY: `T` is transparent over itself
unsafe impl<T> const ReprTransparent<T> for T {}
