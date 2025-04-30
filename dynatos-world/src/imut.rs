//! Inner-mutability types

// Imports
use core::{cell::RefCell, ops};

/// Inner mutability family
pub trait IMutFamily: Sized {
	/// Returns the inner mutability type of `T`
	type IMut<T: ?Sized>: ?Sized + IMutLike<T>;
}

/// Inner mutability-like
pub trait IMutLike<T: ?Sized> {
	/// Reference
	type Ref<'a>: IMutRefLike<'a, T, IMut = Self>
	where
		T: 'a,
		Self: 'a;

	/// Mutable reference
	type RefMut<'a>: IMutRefMutLike<'a, T, IMut = Self>
	where
		T: 'a,
		Self: 'a;

	/// Creates a new value
	fn new(value: T) -> Self
	where
		T: Sized;

	/// Gets a read-lock to this value
	fn read(&self) -> Self::Ref<'_>;

	/// Gets a write-lock to this value
	fn write(&self) -> Self::RefMut<'_>;

	/// Tries to get a read-lock to this value
	fn try_read(&self) -> Option<Self::Ref<'_>>;

	/// Tries to get a write-lock to this value
	fn try_write(&self) -> Option<Self::RefMut<'_>>;
}

/// Inner mutability immutable reference like
pub trait IMutRefLike<'a, T: ?Sized + 'a>: 'a + ops::Deref<Target = T> {
	/// The [`IMutLike`] of this type
	type IMut: ?Sized + IMutLike<T, Ref<'a> = Self>;
}

/// Inner mutability mutable reference like
pub trait IMutRefMutLike<'a, T: ?Sized + 'a>: 'a + ops::DerefMut<Target = T> {
	/// The [`IMutLike`] of this type
	type IMut: ?Sized + IMutLike<T, RefMut<'a> = Self>;

	/// Downgrades this to a [`IMutRefLike`]
	fn downgrade(this: Self) -> <Self::IMut as IMutLike<T>>::Ref<'a>;
}

/// Refcell family of inner-mutability
pub struct StdRefcell;

impl IMutFamily for StdRefcell {
	type IMut<T: ?Sized> = RefCell<T>;
}

impl<T: ?Sized> IMutLike<T> for RefCell<T> {
	type Ref<'a>
		= core::cell::Ref<'a, T>
	where
		Self: 'a;
	type RefMut<'a>
		= RefCellRefMut<'a, T>
	where
		Self: 'a;

	fn new(value: T) -> Self
	where
		T: Sized,
	{
		Self::new(value)
	}

	#[track_caller]
	fn read(&self) -> Self::Ref<'_> {
		self.borrow()
	}

	#[track_caller]
	fn write(&self) -> Self::RefMut<'_> {
		RefCellRefMut {
			borrow:  self.borrow_mut(),
			refcell: self,
		}
	}

	fn try_read(&self) -> Option<Self::Ref<'_>> {
		self.try_borrow().ok()
	}

	fn try_write(&self) -> Option<Self::RefMut<'_>> {
		let borrow = self.try_borrow_mut().ok()?;
		Some(RefCellRefMut { borrow, refcell: self })
	}
}

impl<'a, T: ?Sized> IMutRefLike<'a, T> for core::cell::Ref<'a, T> {
	type IMut = RefCell<T>;
}

/// Wrapper around `core::cell::RefMut`.
#[derive(derive_more::Deref, derive_more::DerefMut, derive_more::Debug)]
#[debug("{borrow:?}")]
pub struct RefCellRefMut<'a, T: ?Sized> {
	/// Borrow
	#[deref(forward)]
	#[deref_mut]
	borrow: core::cell::RefMut<'a, T>,

	/// Original refcell
	// Note: This field is necessary for downgrading.
	refcell: &'a RefCell<T>,
}

impl<'a, T: ?Sized> IMutRefMutLike<'a, T> for RefCellRefMut<'a, T> {
	type IMut = RefCell<T>;

	fn downgrade(this: Self) -> <Self::IMut as IMutLike<T>>::Ref<'a> {
		// Note: RefCell is single threaded, so there are no races here
		drop(this.borrow);
		this.refcell.borrow()
	}
}

/// `parking_lot::RwLock` family of inner-mutability
pub struct ParkingLotRwLock;

impl IMutFamily for ParkingLotRwLock {
	type IMut<T: ?Sized> = parking_lot::RwLock<T>;
}

impl<T: ?Sized> IMutLike<T> for parking_lot::RwLock<T> {
	type Ref<'a>
		= parking_lot::RwLockReadGuard<'a, T>
	where
		Self: 'a;
	type RefMut<'a>
		= parking_lot::RwLockWriteGuard<'a, T>
	where
		Self: 'a;

	fn new(value: T) -> Self
	where
		T: Sized,
	{
		Self::new(value)
	}

	#[track_caller]
	fn read(&self) -> Self::Ref<'_> {
		self.read()
	}

	#[track_caller]
	fn write(&self) -> Self::RefMut<'_> {
		self.write()
	}

	fn try_read(&self) -> Option<Self::Ref<'_>> {
		self.try_read()
	}

	fn try_write(&self) -> Option<Self::RefMut<'_>> {
		self.try_write()
	}
}

impl<'a, T: ?Sized> IMutRefLike<'a, T> for parking_lot::RwLockReadGuard<'a, T> {
	type IMut = parking_lot::RwLock<T>;
}

impl<'a, T: ?Sized> IMutRefMutLike<'a, T> for parking_lot::RwLockWriteGuard<'a, T> {
	type IMut = parking_lot::RwLock<T>;

	fn downgrade(this: Self) -> <Self::IMut as IMutLike<T>>::Ref<'a> {
		Self::downgrade(this)
	}
}
