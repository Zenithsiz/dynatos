//! Loadable value

// Imports
use {
	dynatos_reactive::{SignalGetClone, SignalGetCopy},
	std::{
		convert::Infallible,
		ops::{ControlFlow, Deref, DerefMut, FromResidual, Try},
	},
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

	/// Returns this loadable's value by dereference.
	pub fn as_deref(&self) -> Loadable<&T::Target, E>
	where
		T: Deref,
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

	/// Returns this loadable's value by mutable dereference
	pub fn as_deref_mut(&mut self) -> Loadable<&mut T::Target, E>
	where
		T: DerefMut,
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

impl<T, E> From<Result<T, E>> for Loadable<T, E> {
	fn from(value: Result<T, E>) -> Self {
		match value {
			Ok(value) => Self::Loaded(value),
			Err(err) => Self::Err(err),
		}
	}
}

impl<T, E> From<Option<Result<T, E>>> for Loadable<T, E> {
	fn from(value: Option<Result<T, E>>) -> Self {
		value.map_or(Self::Empty, Self::from)
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

impl<T, E> FromResidual<Option<Infallible>> for Loadable<T, E> {
	fn from_residual(residual: Option<Infallible>) -> Self {
		match residual {
			Some(never) => match never {},
			None => Self::Empty,
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
		for item in iter {
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

impl<T: Copy, E> SignalGetCopy<Loadable<T, E>> for Loadable<&'_ T, E> {
	fn copy_value(self) -> Loadable<T, E> {
		self.map(|value| *value)
	}
}

impl<T: Clone, E> SignalGetClone<Loadable<T, E>> for Loadable<&'_ T, E> {
	fn clone_value(self) -> Loadable<T, E> {
		self.map(|value| value.clone())
	}
}

/// Extension trait for iterators of `Loadable<T, E>`
#[extend::ext(name = IteratorLoadableExt)]
pub impl<I, T, E> I
where
	I: Iterator<Item = Loadable<T, E>>,
{
	/// Flattens an iterator of `Loadable<T, E>` to `Loadable<T::Item, E>`, where `T: IntoIterator`
	fn flatten_loaded(self) -> FlattenLoaded<I, T, E>
	where
		T: IntoIterator,
	{
		FlattenLoaded {
			inner:    self,
			value_it: None,
		}
	}

	/// Finds the position of a value in an iterator of `Loadable<T, E>`.
	fn position_loaded<F>(self, mut pred: F) -> Loadable<Option<usize>, E>
	where
		F: FnMut(T) -> bool,
	{
		for (item_idx, item) in self.enumerate() {
			let item = item?;
			if pred(item) {
				return Loadable::Loaded(Some(item_idx));
			}
		}

		Loadable::Loaded(None)
	}

	/// [`Iterator::scan`]-like adaptor
	fn scan_loaded<St, B, F>(self, init: St, f: F) -> ScanLoaded<I, St, F>
	where
		F: FnMut(&mut St, T) -> Option<B>,
	{
		ScanLoaded {
			inner: self,
			f,
			state: init,
		}
	}
}

/// Iterator returned by [`IteratorLoadableExt::flatten_loaded`]
// TODO: Impl `Clone, Copy, Debug` with the correct bounds.
#[derive(Clone, Copy, Debug)]
pub struct FlattenLoaded<I, T, E>
where
	I: Iterator<Item = Loadable<T, E>>,
	T: IntoIterator,
{
	/// Inner iterator
	inner: I,

	/// Current value iterator
	value_it: Option<T::IntoIter>,
}

impl<I, T, E> Iterator for FlattenLoaded<I, T, E>
where
	I: Iterator<Item = Loadable<T, E>>,
	T: IntoIterator,
{
	type Item = Loadable<T::Item, E>;

	fn next(&mut self) -> Option<Self::Item> {
		// Loop until we find the next value
		loop {
			// If we have a value iterator, try to yield it first
			if let Some(it) = &mut self.value_it {
				match it.next() {
					// If there was still a value, yield it
					Some(value) => return Some(Loadable::Loaded(value)),

					// Otherwise, get rid of the iterator
					None => self.value_it = None,
				}
			}

			// If the inner value didn't have anything, try to get the next value
			match self.inner.next()? {
				// If empty, or error, return them
				Loadable::Empty => return Some(Loadable::Empty),
				Loadable::Err(err) => return Some(Loadable::Err(err)),

				// On loaded, set the value iterator and try to extract it again
				Loadable::Loaded(iter) => self.value_it = Some(iter.into_iter()),
			}
		}
	}
}

/// Iterator returned by [`IteratorLoadableExt::scan_loaded`]
#[derive(Clone, Copy, Debug)]
pub struct ScanLoaded<I, St, F> {
	/// Inner iterator
	inner: I,

	/// Function
	f: F,

	/// State
	state: St,
}

impl<I, T, E, St, B, F> Iterator for ScanLoaded<I, St, F>
where
	I: Iterator<Item = Loadable<T, E>>,
	F: FnMut(&mut St, T) -> Option<B>,
{
	type Item = Loadable<B, E>;

	fn next(&mut self) -> Option<Self::Item> {
		let value = match self.inner.next()? {
			Loadable::Empty => return Some(Loadable::Empty),
			Loadable::Err(err) => return Some(Loadable::Err(err)),
			Loadable::Loaded(value) => value,
		};
		(self.f)(&mut self.state, value).map(Loadable::Loaded)
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
