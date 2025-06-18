//! Multi query

// Imports
use {
	super::{QueriesFn, QueryIntoValue, QueryParse, QueryWrite},
	crate::Location,
	core::{error::Error as StdError, fmt, marker::PhantomData, str::FromStr},
	dynatos_reactive::{Memo, SignalBorrow, SignalBorrowMut},
	std::rc::Rc,
};

/// Parses multiple values from the query
pub struct MultiQuery<T> {
	/// The key to this query
	key: Rc<str>,

	/// Queries with our key
	queries: Memo<Vec<String>, QueriesFn>,

	/// Phantom
	_phantom: PhantomData<fn() -> T>,
}

impl<T> MultiQuery<T> {
	/// Creates a new query
	pub fn new(key: impl Into<Rc<str>>) -> Self {
		let key = key.into();
		Self {
			key:      Rc::clone(&key),
			queries:  super::queries_memo(key),
			_phantom: PhantomData,
		}
	}

	/// Returns the key to this query
	#[must_use]
	pub fn key(&self) -> &str {
		&self.key
	}
}

impl<T> Clone for MultiQuery<T> {
	fn clone(&self) -> Self {
		Self {
			key: Rc::clone(&self.key),
			queries: self.queries.clone(),
			..*self
		}
	}
}

impl<T: FromStr> QueryParse for MultiQuery<T> {
	type Value = Result<Vec<T>, QueryParseError<T>>;

	fn parse(&self) -> Self::Value {
		let queries = self.queries.borrow();
		queries
			.iter()
			.enumerate()
			.map(|(idx, value)| match value.parse::<T>() {
				Ok(value) => Ok(value),
				Err(err) => Err(QueryParseError {
					idx,
					value: value.clone(),
					err,
				}),
			})
			.collect()
	}
}

impl<T: FromStr> QueryIntoValue<Vec<T>> for MultiQuery<T> {
	fn into_query_value(value: Vec<T>) -> Self::Value {
		Ok(value)
	}
}

impl<T: FromStr<Err: StdError> + ToString> QueryWrite<&'_ Result<Vec<T>, QueryParseError<T>>> for MultiQuery<T> {
	fn write(&self, new_value: &Result<Vec<T>, QueryParseError<T>>) {
		match new_value {
			Ok(new_value) => self.write(&**new_value),
			Err(err) => tracing::warn!(?self.key, ?err, "Cannot assign an error to a query value"),
		}
	}
}

impl<T: FromStr<Err: StdError> + ToString> QueryWrite<&[T]> for MultiQuery<T> {
	fn write(&self, new_value: &[T]) {
		// Update our queries memo manually and prevent it from being added
		let _suppress_queries = self.queries.suppress();
		self.queries.update_raw(new_value.iter().map(T::to_string).collect());

		dynatos_context::with_expect::<Location, _, _>(|location| {
			let mut location = location.borrow_mut();
			let mut added_query = false;
			let mut queries = vec![];
			for (key, value) in location.query_pairs().into_owned() {
				// If it's another key, keep it
				if key != *self.key {
					queries.push((key, value));
					continue;
				}

				// If we already added our query, this is a duplicate, so skip it
				if added_query {
					continue;
				}

				// If it's our key, add all values
				added_query = true;
				queries.extend(
					new_value
						.iter()
						.map(T::to_string)
						.map(|value| (self.key.to_string(), value)),
				);
			}

			// If we haven't added ours yet by now, add it at the end
			if !added_query {
				queries.extend(
					new_value
						.iter()
						.map(T::to_string)
						.map(|value| (self.key.to_string(), value)),
				);
			}

			location.query_pairs_mut().clear().extend_pairs(queries);
		});
	}
}

/// Error for `Vec<T>` impl of [`FromQuery`]
#[derive(thiserror::Error)]
#[error("Unable to parse argument {idx}: {value:?}")]
pub struct QueryParseError<T: FromStr> {
	/// Index we were unable to parse
	idx: usize,

	/// Value we were unable to parse
	value: String,

	/// Inner error
	#[source]
	err: T::Err,
}

impl<T> fmt::Debug for QueryParseError<T>
where
	T: FromStr,
	T::Err: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("VecFromQueryError")
			.field("idx", &self.idx)
			.field("value", &self.value)
			.field("err", &self.err)
			.finish()
	}
}
