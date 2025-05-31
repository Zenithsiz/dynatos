//! `Option<T>` Signal with default value

// Imports
use {
	crate::{
		SignalBorrow,
		SignalBorrowMut,
		SignalGetClonedDefaultImpl,
		SignalGetDefaultImpl,
		SignalReplace,
		SignalSet,
		SignalSetDefaultImpl,
		SignalUpdate,
		SignalUpdateDefaultImpl,
		SignalWith,
		SignalWithDefaultImpl,
	},
	core::ops::{Deref, DerefMut},
};

/// Wrapper for a `Signal<Option<T>>` with a default value
#[derive(Clone, Debug)]
pub struct WithDefault<S, T: ?Sized> {
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

	fn borrow(&self) -> Self::Ref<'_> {
		BorrowRef {
			value:   self.inner.borrow(),
			default: &self.default,
		}
	}

	fn borrow_raw(&self) -> Self::Ref<'_> {
		BorrowRef {
			value:   self.inner.borrow_raw(),
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

	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		self.inner.with(|value| match value.into() {
			Some(value) => f(value),
			None => f(&self.default),
		})
	}

	fn with_raw<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		self.inner.with_raw(|value| match value.into() {
			Some(value) => f(value),
			None => f(&self.default),
		})
	}
}

// Note: We disable the default impl because we can impl `SignalSet<T>` more
//       efficiently and with less bounds since we don't need to update the
//       value with the default, just to overwrite it afterwards.
impl<S, T: ?Sized> !SignalSetDefaultImpl for WithDefault<S, T> {}

// TODO: Do the defaults work? Or should we overwrite them?
impl<S, T: ?Sized> SignalGetDefaultImpl for WithDefault<S, T> {}
impl<S, T: ?Sized> SignalGetClonedDefaultImpl for WithDefault<S, T> {}


// Note: We disable the default impl because we can impl `SignalWith<T>` for
//       more signals (e.g. those that only impl `SignalWith` and not `SignalBorrow`)
impl<S, T: ?Sized> !SignalWithDefaultImpl for WithDefault<S, T> {}

// Note: We disable the default impl because we can impl `SignalUpdate<T>` for
//       more signals (e.g. those that only impl `SignalUpdate` and not `SignalBorrowMut`)
impl<S, T: ?Sized> !SignalUpdateDefaultImpl for WithDefault<S, T> {}

impl<S, T> SignalSet<T> for WithDefault<S, T>
where
	S: SignalSet<Option<T>>,
{
	fn set(&self, new_value: T) {
		self.inner.set(Some(new_value));
	}

	fn set_raw(&self, new_value: T) {
		self.inner.set_raw(Some(new_value));
	}
}

impl<S, T> SignalSet<Option<T>> for WithDefault<S, T>
where
	S: SignalSet<Option<T>>,
{
	fn set(&self, new_value: Option<T>) {
		self.inner.set(new_value);
	}

	fn set_raw(&self, new_value: Option<T>) {
		self.inner.set_raw(new_value);
	}
}

impl<S, T> SignalReplace<T> for WithDefault<S, T>
where
	S: SignalReplace<Option<T>, Value = Option<T>>,
	T: Copy,
{
	type Value = T;

	fn replace(&self, new_value: T) -> Self::Value {
		self.inner.replace(Some(new_value)).unwrap_or(self.default)
	}

	fn replace_raw(&self, new_value: T) -> Self::Value {
		self.inner.replace_raw(Some(new_value)).unwrap_or(self.default)
	}
}

impl<S, T> SignalReplace<Option<T>> for WithDefault<S, T>
where
	S: SignalReplace<Option<T>, Value = Option<T>>,
	T: Copy,
{
	type Value = T;

	fn replace(&self, new_value: Option<T>) -> Self::Value {
		self.inner.replace(new_value).unwrap_or(self.default)
	}

	fn replace_raw(&self, new_value: Option<T>) -> Self::Value {
		self.inner.replace_raw(new_value).unwrap_or(self.default)
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

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let mut value = self.inner.borrow_mut();
		value.get_or_insert(self.default);

		BorrowRefMut { value }
	}

	fn borrow_mut_raw(&self) -> Self::RefMut<'_> {
		let mut value = self.inner.borrow_mut_raw();
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

	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		self.inner.update(|value| f(value.get_or_insert(self.default)))
	}

	fn update_raw<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		self.inner.update_raw(|value| f(value.get_or_insert(self.default)))
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
