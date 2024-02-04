//! `Option<T>` Signal with default value

// Imports
use crate::{SignalGet, SignalSet, SignalUpdate, SignalWith};

/// Wrapper for a `Signal<Option<T>>` with a default value
#[derive(Clone)]
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

impl<S, T> SignalGet for WithDefault<S, T>
where
	S: SignalGet<Value = Option<T>>,
	T: Copy,
{
	type Value = T;

	fn get(&self) -> Self::Value {
		self.inner.get().unwrap_or(self.default)
	}
}

impl<S, T> SignalWith for WithDefault<S, T>
where
	S: SignalWith<Value = Option<T>>,
{
	type Value = T;

	fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Self::Value) -> O,
	{
		self.inner.with(|value| match value {
			Some(value) => f(value),
			None => f(&self.default),
		})
	}
}

// TODO: Requiring `T: Copy` just because of the return is a bit awkward.
//       Should we make `SignalSet` not return the previous value? Or at least
//       move it to a different trait like `SignalReplace` or something?
impl<S, T> SignalSet for WithDefault<S, T>
where
	S: SignalSet<Value = Option<T>>,
	T: Copy,
{
	type Value = T;

	fn set(&self, new_value: Self::Value) -> Self::Value {
		self.inner.set(Some(new_value)).unwrap_or(self.default)
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
