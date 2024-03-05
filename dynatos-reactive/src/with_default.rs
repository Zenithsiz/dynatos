//! `Option<T>` Signal with default value

// Imports
use crate::{SignalReplace, SignalUpdate, SignalWith};

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
	pub fn new(inner: S, default: T) -> Self {
		Self { inner, default }
	}
}

impl<S, T> SignalWith for WithDefault<S, T>
where
	S: for<'a> SignalWith<Value<'a> = Option<&'a T>>,
	T: 'static,
{
	type Value<'a> = &'a T;

	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		self.inner.with(|value| match value {
			Some(value) => f(value),
			None => f(&self.default),
		})
	}
}

impl<S, T> SignalReplace<T> for WithDefault<S, T>
where
	S: SignalReplace<Option<T>>,
	T: Copy,
{
	fn replace(&self, new_value: T) -> T {
		self.inner.replace(Some(new_value)).unwrap_or(self.default)
	}
}

impl<S, T> SignalReplace<Option<T>> for WithDefault<S, T>
where
	S: SignalReplace<Option<T>>,
{
	fn replace(&self, new_value: Option<T>) -> Option<T> {
		self.inner.replace(new_value)
	}
}

impl<S, T> SignalUpdate for WithDefault<S, T>
where
	S: SignalUpdate<Value = Option<T>>,
	T: Copy,
{
	type Value = T;

	fn update<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&mut Self::Value) -> O,
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
