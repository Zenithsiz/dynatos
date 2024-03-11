//! Segmented Vector

// Imports
use std::{
	cell::{Cell, UnsafeCell},
	mem::MaybeUninit,
};

/// Segmented Vector
pub struct SegVec<T, const N: usize = 8> {
	/// Segments
	segments: UnsafeCell<Vec<Box<[MaybeUninit<T>; N]>>>,

	/// Length
	len: Cell<usize>,
}

impl<T, const N: usize> SegVec<T, N> {
	/// Creates a new, empty, segmented vector
	#[must_use]
	pub fn new() -> Self {
		Self {
			segments: UnsafeCell::new(vec![]),
			len:      Cell::new(0),
		}
	}

	/// Returns the length of this vector
	pub fn len(&self) -> usize {
		self.len.get()
	}

	/// Returns if this vector is empty
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Gets an element from this vector
	pub fn get(&self, idx: usize) -> Option<&T> {
		// If the index is beyond our length, it's uninitialized / missing.
		if idx >= self.len.get() {
			return None;
		}

		// SAFETY: No mutable references exist to any element since our
		//         receiver is `&self`. We also only access values immutably
		//         through this pointer.
		let segments = unsafe { &*self.segments.get() };

		// Find the value.
		let segment_idx = idx / N;
		let value_idx = idx % N;
		let value = &segments[segment_idx][value_idx];

		// SAFETY: We know that value is initialized, given that it's index is
		//         within our length.
		Some(unsafe { value.assume_init_ref() })
	}

	/// Gets an element mutably
	pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
		// If the index is beyond our length, it's uninitialized / missing.
		if idx >= self.len.get() {
			return None;
		}

		// Find the value
		let segments = self.segments.get_mut();
		let segment_idx = idx / N;
		let value_idx = idx % N;
		let value = &mut segments[segment_idx][value_idx];

		// SAFETY: We know that value is initialized, given that it's index is
		//         within our length.
		Some(unsafe { value.assume_init_mut() })
	}

	/// Pushes an element into this segmented vector
	pub fn push(&self, value: T) -> &T {
		// SAFETY: We never hand out references to the segments, so no other borrow exists
		//         at this time.
		//         We also don't access any live values through this pointer.
		let segments = unsafe { &mut *self.segments.get() };

		// If we've reached the end of the last segment, allocate a new segment
		if self.len.get() == segments.len() * N {
			let segment = Box::new([const { MaybeUninit::uninit() }; N]);
			segments.push(segment);
		}

		// Then write the value and update the length.
		let segment_idx = self.len.get() / N;
		let value_idx = self.len.get() % N;
		let value = segments[segment_idx][value_idx].write(value);
		self.len.update(|len| len + 1);

		value
	}
}

impl<T, const N: usize> Default for SegVec<T, N> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: std::fmt::Debug, const N: usize> std::fmt::Debug for SegVec<T, N> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let entries = (0..self.len.get()).map(|idx| self.get(idx).expect("Index was invalid"));
		f.debug_list().entries(entries).finish()
	}
}

impl<T, const N: usize> Drop for SegVec<T, N> {
	fn drop(&mut self) {
		let segments = self.segments.get_mut();
		let len = self.len.get();
		for segment in segments {
			for value_idx in 0..len % N {
				let value = &mut segment[value_idx];

				// SAFETY: We know that value is initialized, given that it's index is
				//         within our length.
				unsafe {
					value.assume_init_drop();
				}
			}
		}
	}
}


#[cfg(test)]
mod test {
	use {super::*, std::ptr};

	#[test]
	fn push_no_invalidate() {
		let vec: SegVec<i32> = SegVec::new();
		let a = vec.push(0);
		for idx in 1..=8 {
			vec.push(idx);
		}

		assert_eq!(*a, 0);
	}

	#[test]
	fn push_get() {
		let vec: SegVec<i32> = SegVec::new();
		let a1 = vec.push(1);
		let a2 = vec.get(0).expect("Unable to get value");

		assert_eq!(ptr::from_ref(a1), ptr::from_ref(a2));
	}

	#[test]
	fn push_get_mut() {
		let mut vec: SegVec<i32> = SegVec::new();
		let a1 = vec.push(1) as *const _;
		let a2 = vec.get_mut(0).expect("Unable to get value");

		assert_eq!(a1, ptr::from_ref(a2));
	}
}
