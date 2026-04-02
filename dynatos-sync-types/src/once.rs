//! Once cell

type Inner<T> = cfg_select! {
	feature = "sync" => std::sync::OnceLock::<T>,
	_ => core::cell::OnceCell::<T>,
};

pub struct OnceCell<T>(Inner<T>);

impl<T> OnceCell<T> {
	#[must_use]
	pub const fn new() -> Self {
		Self(Inner::new())
	}

	pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
		self.0.get_or_init(f)
	}

	pub fn get(&self) -> Option<&T> {
		self.0.get()
	}

	pub fn set(&self, value: T) -> Result<(), T> {
		self.0.set(value)
	}
}

impl<T> Default for OnceCell<T> {
	fn default() -> Self {
		Self::new()
	}
}
