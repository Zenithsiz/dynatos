//! Reference-counted pointer

// Imports
use {
	core::ops,
	std::{rc, sync},
};

/// Reference-counted pointer family
pub trait RcFamily: Sized {
	/// Returns the reference counted type of `T`
	type Rc<T: ?Sized>: RcLike<T, Family = Self>;

	/// Weak type
	type Weak<T: ?Sized>: WeakLike<T, Family = Self>;
}

/// A reference-counted pointer
pub trait RcLike<T: ?Sized>: ops::Deref<Target = T> + Clone {
	/// The family of this pointer
	type Family: RcFamily<Rc<T> = Self>;

	/// Creates a new Rc from a value
	fn new(value: T) -> Self
	where
		T: Sized;

	/// Downgrades this Rc to a Weak
	fn downgrade(this: &Self) -> <Self::Family as RcFamily>::Weak<T>;

	/// Gets a pointer to the inner data of this Rc
	fn as_ptr(this: &Self) -> *const T;

	/// Returns the strong count of this Rc
	fn strong_count(this: &Self) -> usize;

	/// Returns the weak count of this Rc
	fn weak_count(this: &Self) -> usize;
}

/// A Reference-counted weak pointer
pub trait WeakLike<T: ?Sized>: Clone {
	/// The family of this pointer
	type Family: RcFamily<Weak<T> = Self>;

	/// Upgrades this weak to an rc
	fn upgrade(&self) -> Option<<Self::Family as RcFamily>::Rc<T>>;

	/// Gets a pointer to the inner data of this rc
	fn as_ptr(&self) -> *const T;
}

/// Arc family of reference-counter pointers
pub struct StdArc;

impl RcFamily for StdArc {
	type Rc<T: ?Sized> = sync::Arc<T>;
	type Weak<T: ?Sized> = sync::Weak<T>;
}

impl<T: ?Sized> RcLike<T> for sync::Arc<T> {
	type Family = StdArc;

	fn new(value: T) -> Self
	where
		T: Sized,
	{
		Self::new(value)
	}

	fn downgrade(this: &Self) -> <Self::Family as RcFamily>::Weak<T> {
		Self::downgrade(this)
	}

	fn as_ptr(this: &Self) -> *const T {
		Self::as_ptr(this)
	}

	fn strong_count(this: &Self) -> usize {
		Self::strong_count(this)
	}

	fn weak_count(this: &Self) -> usize {
		Self::weak_count(this)
	}
}

impl<T: ?Sized> WeakLike<T> for sync::Weak<T> {
	type Family = StdArc;

	fn upgrade(&self) -> Option<<Self::Family as RcFamily>::Rc<T>> {
		self.upgrade()
	}

	fn as_ptr(&self) -> *const T {
		self.as_ptr()
	}
}

/// Rc family of reference-counter pointers
pub struct StdRc;

impl RcFamily for StdRc {
	type Rc<T: ?Sized> = rc::Rc<T>;
	type Weak<T: ?Sized> = rc::Weak<T>;
}

impl<T: ?Sized> RcLike<T> for rc::Rc<T> {
	type Family = StdRc;

	fn new(value: T) -> Self
	where
		T: Sized,
	{
		Self::new(value)
	}

	fn downgrade(this: &Self) -> <Self::Family as RcFamily>::Weak<T> {
		Self::downgrade(this)
	}

	fn as_ptr(this: &Self) -> *const T {
		Self::as_ptr(this)
	}

	fn strong_count(this: &Self) -> usize {
		Self::strong_count(this)
	}

	fn weak_count(this: &Self) -> usize {
		Self::weak_count(this)
	}
}

impl<T: ?Sized> WeakLike<T> for rc::Weak<T> {
	type Family = StdRc;

	fn upgrade(&self) -> Option<<Self::Family as RcFamily>::Rc<T>> {
		self.upgrade()
	}

	fn as_ptr(&self) -> *const T {
		self.as_ptr()
	}
}
