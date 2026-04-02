//! Read-write inner mutability type

// Imports
use core::ops::{Deref, DerefMut};

type Inner<T> = cfg_select! {
	feature = "sync" => std::sync::nonpoison::RwLock::<T>,
	_ => core::cell::RefCell::<T>,
};

type RefInner<'a, T> = cfg_select! {
	feature = "sync" => std::sync::nonpoison::RwLockReadGuard::<'a, T>,
	_ => core::cell::Ref::<'a, T>,
};

type RefMutInner<'a, T> = cfg_select! {
	feature = "sync" => std::sync::nonpoison::RwLockWriteGuard::<'a, T>,
	_ => core::cell::RefMut::<'a, T>,
};

/// Read-Write inner mutability
#[derive(Default, Debug)]
pub struct IMutRw<T: ?Sized>(Inner<T>);

impl<T> IMutRw<T> {
	pub const fn new(value: T) -> Self {
		Self(Inner::new(value))
	}
}

impl<T: ?Sized> IMutRw<T> {
	pub fn read(&self) -> IMutRwRef<'_, T> {
		IMutRwRef(cfg_select! {
			feature = "sync" => self.0.read(),
			_ => self.0.borrow(),
		})
	}

	pub fn try_read(&self) -> Result<IMutRwRef<'_, T>, ReadError> {
		#[expect(clippy::map_err_ignore, reason = "The error is a ZST we don't care about")]
		cfg_select! {
			feature = "sync" => self.0.try_read(),
			_ => self.0.try_borrow(),
		}
		.map(IMutRwRef)
		.map_err(|_| ReadError)
	}

	pub fn write(&self) -> IMutRwRefMut<'_, T> {
		IMutRwRefMut(cfg_select! {
			feature = "sync" => self.0.write(),
			_ => self.0.borrow_mut(),
		})
	}

	pub fn try_write(&self) -> Result<IMutRwRefMut<'_, T>, WriteError> {
		#[expect(clippy::map_err_ignore, reason = "The error is a ZST we don't care about")]
		cfg_select! {
			feature = "sync" => self.0.try_write(),
			_ => self.0.try_borrow_mut(),
		}
		.map(IMutRwRefMut)
		.map_err(|_| WriteError)
	}
}

#[derive(Debug)]
pub struct ReadError;

#[derive(Debug)]
pub struct WriteError;

#[derive(Debug)]
pub struct IMutRwRef<'a, T: ?Sized>(RefInner<'a, T>);

impl<T: ?Sized> Deref for IMutRwRef<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Debug)]
pub struct IMutRwRefMut<'a, T: ?Sized>(RefMutInner<'a, T>);

impl<T: ?Sized> Deref for IMutRwRefMut<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: ?Sized> DerefMut for IMutRwRefMut<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}
