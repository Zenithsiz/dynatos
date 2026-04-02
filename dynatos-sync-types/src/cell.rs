//! Cell types

// Imports
use core::sync::atomic;

duplicate::duplicate! {
	[
		Name Inner AtomicTy InnerTy;
		[CellBool] [InnerBool] [AtomicBool] [bool];
		[CellUsize] [InnerUsize] [AtomicUsize] [usize];
	]

	type Inner = cfg_select! {
		feature = "sync" => core::sync::atomic::AtomicTy,
		_ => core::cell::Cell::<InnerTy>,
	};

	/// Cell
	pub struct Name(Inner);

	impl Name {
		#[must_use]
		pub const fn new(value: InnerTy) -> Self {
			Self(Inner::new(value))
		}

		#[cfg_attr(
			not(feature = "sync"),
			expect(clippy::missing_const_for_fn, reason = "Used with the feature")
		)]
		pub fn get(&self, ordering: atomic::Ordering) -> InnerTy {
			cfg_select! {
				feature = "sync" => self.0.load(ordering),
				_ => {
					let _: atomic::Ordering = ordering;
					self.0.get()
				},
			}
		}

		pub fn set(&self, value: InnerTy, ordering: atomic::Ordering) {
			cfg_select! {
				feature = "sync" => self.0.store(value, ordering),
				_ => {
					let _: atomic::Ordering = ordering;
					self.0.set(value)
				},
			};
		}

		pub fn swap(&self, value: InnerTy, ordering: atomic::Ordering) -> InnerTy {
			cfg_select! {
				feature = "sync" => self.0.swap(value, ordering),
				_ => {
					let _: atomic::Ordering = ordering;
					let old_value = self.0.get();
					self.0.set(value);
					old_value
				},
			}
		}
	}
}
