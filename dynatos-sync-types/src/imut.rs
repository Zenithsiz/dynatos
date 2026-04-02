//! Inner mutability type

// Imports
use core::ops::{Deref, DerefMut};

type Inner<T> = cfg_select! {
	feature = "sync" => std::sync::nonpoison::Mutex::<T>,
	_ => core::cell::RefCell::<T>,
};

type RefInner<'a, T> = cfg_select! {
	feature = "sync" => std::sync::nonpoison::MutexGuard::<'a, T>,
	_ => core::cell::RefMut::<'a, T>,
};

/// Write-only inner mutability
#[derive(Default, Debug)]
pub struct IMut<T: ?Sized>(Inner<T>);

impl<T> IMut<T> {
	pub const fn new(value: T) -> Self {
		Self(Inner::new(value))
	}
}

impl<T: ?Sized> IMut<T> {
	pub fn lock(&self) -> IMutRef<'_, T> {
		IMutRef(cfg_select! {
			feature = "sync" => self.0.lock(),
			_ => self.0.borrow_mut(),
		})
	}
}

#[derive(Debug)]
pub struct IMutRef<'a, T: ?Sized>(pub(crate) RefInner<'a, T>);

impl<T: ?Sized> Deref for IMutRef<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: ?Sized> DerefMut for IMutRef<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}
