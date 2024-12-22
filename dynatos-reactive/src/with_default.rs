//! `Option<T>` Signal with default value

// Imports
use {
	crate::{SignalBorrow, SignalBorrowMut, SignalReplace, SignalUpdate, SignalWith},
	core::ops::{Deref, DerefMut},
};

/// Wrapper for a `Signal<Option<T>>` with a default value
#[derive(Clone, Debug)]
pub struct WithDefault<S, T> {
	/// Inner signal
	inner: S,

	/// Default
	default: T,
}

impl<S, T> WithDefault<S, T> {
	/// Wraps a signal with a default value
	pub const fn new(inner: S, default: T) -> Self {
		Self { inner, default }
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, S: SignalBorrow + 'a, T> {
	/// value
	value: S::Ref<'a>,

	/// Default value
	default: &'a T,
}

impl<'a, S, T> Deref for BorrowRef<'a, S, T>
where
	S: SignalBorrow + 'a,
	S::Ref<'a>: Deref<Target = Option<T>>,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		match &*self.value {
			Some(value) => value,
			None => self.default,
		}
	}
}

impl<S: SignalBorrow, T> SignalBorrow for WithDefault<S, T> {
	type Ref<'a>
		= BorrowRef<'a, S, T>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		BorrowRef {
			value:   self.inner.borrow(),
			default: &self.default,
		}
	}
}

impl<S, T> SignalWith for WithDefault<S, T>
where
	S: SignalWith,
	// Note: This allows both `Option<&'_ T>` and `&'_ Option<T>`
	for<'a> S::Value<'a>: Into<Option<&'a T>>,
	T: 'static,
{
	type Value<'a> = &'a T;

	#[track_caller]
	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		self.inner.with(|value| match value.into() {
			Some(value) => f(value),
			None => f(&self.default),
		})
	}
}

// TODO: Impl `SignalGet<Option<T>>` once we can?

impl<S, T> SignalReplace<T> for WithDefault<S, T>
where
	S: SignalReplace<Option<T>>,
	T: Copy,
{
	#[track_caller]
	fn replace(&self, new_value: T) -> T {
		self.inner.replace(Some(new_value)).unwrap_or(self.default)
	}
}

impl<S, T> SignalReplace<Option<T>> for WithDefault<S, T>
where
	S: SignalReplace<Option<T>>,
{
	#[track_caller]
	fn replace(&self, new_value: Option<T>) -> Option<T> {
		self.inner.replace(new_value)
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a, S: SignalBorrowMut + 'a> {
	/// value
	value: S::RefMut<'a>,
}

impl<'a, S, T> Deref for BorrowRefMut<'a, S>
where
	S: SignalBorrowMut + 'a,
	S::RefMut<'a>: Deref<Target = Option<T>>,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.value.as_ref().expect("Default value was missing")
	}
}

impl<'a, S, T> DerefMut for BorrowRefMut<'a, S>
where
	S: SignalBorrowMut + 'a,
	S::RefMut<'a>: DerefMut<Target = Option<T>>,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.value.as_mut().expect("Default value was missing")
	}
}

impl<S: SignalBorrowMut, T> SignalBorrowMut for WithDefault<S, T>
where
	for<'a> S::RefMut<'a>: DerefMut<Target = Option<T>>,
	T: Copy,
{
	type RefMut<'a>
		= BorrowRefMut<'a, S>
	where
		Self: 'a;

	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let mut value = self.inner.borrow_mut();
		value.get_or_insert(self.default);

		BorrowRefMut { value }
	}
}

impl<S, T> SignalUpdate for WithDefault<S, T>
where
	S: for<'a> SignalUpdate<Value<'a> = &'a mut Option<T>>,
	T: Copy + 'static,
{
	type Value<'a> = &'a mut T;

	#[track_caller]
	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		self.inner.update(|value| f(value.get_or_insert(self.default)))
	}
}

/// Extension trait to add a default value to a signal
#[extend::ext_sized(name = SignalWithDefault)]
pub impl<S> S {
	/// Wraps this signal with a default value
	fn with_default<T>(self, default: T) -> WithDefault<S, T> {
		WithDefault::new(self, default)
	}
}
