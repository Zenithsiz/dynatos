//! Lazy cell

use core::ops::Deref;

type Inner<T, F> = cfg_select! {
	feature = "sync" => std::sync::LazyLock::<T, F>,
	_ => core::cell::LazyCell::<T, F>,
};

pub struct LazyCell<T, F = fn() -> T>(Inner<T, F>);

impl<T, F> LazyCell<T, F> {
	#[must_use]
	pub const fn new(f: F) -> Self
	where
		F: FnOnce() -> T,
	{
		Self(Inner::new(f))
	}
}

impl<T, F> Deref for LazyCell<T, F>
where
	F: FnOnce() -> T,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
