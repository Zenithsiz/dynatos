//! Loadable value

// Imports
use std::{
	convert::Infallible,
	ops::{ControlFlow, FromResidual, Try},
};

/// Loadable value.
#[derive(Clone, Copy, Debug)]
pub enum Loadable<T, E> {
	/// Empty
	Empty,

	/// Failed to load
	Err(E),

	/// Loaded
	Loaded(T),
}

impl<T, E> Loadable<T, E> {
	/// Creates a loadable from a result
	pub fn from_res<E2>(res: Result<T, E2>) -> Self
	where
		E: From<E2>,
	{
		match res {
			Ok(value) => Self::Loaded(value),
			Err(err) => Self::Err(err.into()),
		}
	}

	/// Returns if the loadable is empty.
	#[must_use]
	pub fn is_empty(&self) -> bool {
		matches!(self, Self::Empty)
	}

	/// Returns if the loadable is loaded.
	///
	/// This means it's either an error or a value
	#[must_use]
	pub fn is_loaded(&self) -> bool {
		!self.is_empty()
	}

	/// Returns this loadable's value by reference.
	pub fn as_ref(&self) -> Loadable<&T, E>
	where
		E: Clone,
	{
		match self {
			Self::Empty => Loadable::Empty,
			Self::Err(err) => Loadable::Err(err.clone()),
			Self::Loaded(value) => Loadable::Loaded(value),
		}
	}

	/// Returns this loadable's value by mutable reference
	pub fn as_mut(&mut self) -> Loadable<&mut T, E>
	where
		E: Clone,
	{
		match self {
			Self::Empty => Loadable::Empty,
			Self::Err(err) => Loadable::Err(err.clone()),
			Self::Loaded(value) => Loadable::Loaded(value),
		}
	}

	/// Maps this loadable's value
	pub fn map<U, F>(self, f: F) -> Loadable<U, E>
	where
		F: FnOnce(T) -> U,
	{
		match self {
			Self::Empty => Loadable::Empty,
			Self::Err(err) => Loadable::Err(err),
			Self::Loaded(value) => Loadable::Loaded(f(value)),
		}
	}

	/// Zips two loadable.
	///
	/// If is empty, the result will be empty.
	/// If any is errored, the result will be an error.
	pub fn zip<U>(self, rhs: Loadable<U, E>) -> Loadable<(T, U), E> {
		match (self, rhs) {
			// If there's an error, propagate
			(Self::Err(err), _) | (_, Loadable::Err(err)) => Loadable::Err(err),

			// Otherwise, if we have both values, return loaded
			(Self::Loaded(lhs), Loadable::Loaded(rhs)) => Loadable::Loaded((lhs, rhs)),

			// Otherwise, we're empty
			_ => Loadable::Empty,
		}
	}

	/// Chains this loadable with another if it's loaded
	///
	/// If any operation returns empty or error, it will be propagated
	pub fn and_then<U, F>(self, f: F) -> Loadable<U, E>
	where
		F: FnOnce(T) -> Loadable<U, E>,
	{
		match self {
			Self::Empty => Loadable::Empty,
			Self::Err(err) => Loadable::Err(err),
			Self::Loaded(value) => f(value),
		}
	}

	/// Converts this to an option.
	///
	/// Maps `Loadable::Loaded` to `Some` and the rest to `None`.
	pub fn loaded(self) -> Option<T> {
		match self {
			Self::Empty => None,
			Self::Err(_err) => None,
			Self::Loaded(value) => Some(value),
		}
	}
}

impl<T, E> Loadable<&T, E> {
	/// Clones the inner value
	pub fn cloned(self) -> Loadable<T, E>
	where
		T: Clone,
	{
		self.map(T::clone)
	}
}

impl<T, E> From<T> for Loadable<T, E> {
	fn from(value: T) -> Self {
		Self::Loaded(value)
	}
}

impl<T, E> Try for Loadable<T, E> {
	type Output = T;
	type Residual = Loadable<!, E>;

	fn from_output(output: Self::Output) -> Self {
		Self::Loaded(output)
	}

	fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
		match self {
			Self::Empty => ControlFlow::Break(Loadable::Empty),
			Self::Err(err) => ControlFlow::Break(Loadable::Err(err)),
			Self::Loaded(value) => ControlFlow::Continue(value),
		}
	}
}

impl<T, E, E2> FromResidual<Loadable<!, E2>> for Loadable<T, E>
where
	E: From<E2>,
{
	fn from_residual(residual: Loadable<!, E2>) -> Self {
		match residual {
			Loadable::Empty => Self::Empty,
			Loadable::Err(err) => Self::Err(err.into()),
			Loadable::Loaded(never) => never,
		}
	}
}

impl<T, E, E2> FromResidual<Result<Infallible, E2>> for Loadable<T, E>
where
	E: From<E2>,
{
	fn from_residual(residual: Result<Infallible, E2>) -> Self {
		match residual {
			Ok(never) => match never {},
			Err(err) => Self::Err(err.into()),
		}
	}
}

/// Collects an iterator of `Loadable<T, E>` into a `Loadable<C, E>`,
/// where `C` is a collection of `T`s.
///
/// If any empty, or error loadables are found, this immediately short-circuits
/// and returns them
impl<C, T, E> FromIterator<Loadable<T, E>> for Loadable<C, E>
where
	C: Default + Extend<T>,
{
	fn from_iter<I: IntoIterator<Item = Loadable<T, E>>>(iter: I) -> Self {
		let mut collection = C::default();
		for item in iter.into_iter() {
			// If we find any empty, or errors, return them immediately
			let item = match item {
				Loadable::Empty => return Self::Empty,
				Loadable::Err(err) => return Self::Err(err),
				Loadable::Loaded(value) => value,
			};

			collection.extend_one(item);
		}

		Self::Loaded(collection)
	}
}

/// Extension trait to create a [`Loadable::Loaded`] from a value.
#[extend::ext(name = IntoLoaded)]
pub impl<T> T {
	/// Converts this `T` value into a loaded `Loadable<T>`
	fn into_loaded<E>(self) -> Loadable<T, E> {
		Loadable::Loaded(self)
	}
}
