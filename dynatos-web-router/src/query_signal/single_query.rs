//! Single query

// Imports
use {
	super::{QueriesFn, QueryIntoValue, QueryParse, QueryWrite},
	crate::LocationSignal,
	core::{error::Error as StdError, marker::PhantomData, str::FromStr},
	dynatos_loadable::Loadable,
	dynatos_reactive::{Memo, SignalBorrow, SignalBorrowMut},
	dynatos_sync_types::{RcPtr, SyncBounds},
	dynatos_web::DynatosWebCtx,
};

/// Parses a singular value from the query.
///
/// Requires a value of type [`LocationSignal`](crate::LocationSignal) in the context store.
pub struct SingleQuery<T> {
	/// The key to this query
	key: RcPtr<str>,

	/// Queries with our key
	queries: Memo<Vec<String>, QueriesFn>,

	ctx:      DynatosWebCtx,
	_phantom: PhantomData<fn() -> T>,
}

impl<T> SingleQuery<T> {
	/// Creates a new query
	pub fn new(ctx: &DynatosWebCtx, key: impl Into<RcPtr<str>>) -> Self {
		let key = key.into();
		Self {
			key:      RcPtr::clone(&key),
			queries:  super::queries_memo(ctx, key),
			ctx:      ctx.clone(),
			_phantom: PhantomData,
		}
	}

	/// Returns the key to this query
	#[must_use]
	pub fn key(&self) -> &str {
		&self.key
	}
}

impl<T> Clone for SingleQuery<T> {
	fn clone(&self) -> Self {
		Self {
			key:      RcPtr::clone(&self.key),
			queries:  self.queries.clone(),
			ctx:      self.ctx.clone(),
			_phantom: PhantomData,
		}
	}
}

impl<T> QueryParse for SingleQuery<T>
where
	T: SyncBounds + FromStr,
	T::Err: SyncBounds,
{
	type Value = Loadable<T, T::Err>;

	fn parse(&self) -> Self::Value {
		let queries = self.queries.borrow();
		let value = match &**queries {
			[] => return Loadable::Empty,
			[value] => value,
			[first, rest @ ..] => {
				tracing::warn!(?self.key, ?first, ?rest, "Ignoring duplicate queries, using first");
				first
			},
		};

		value.parse::<T>().into()
	}
}

impl<T> QueryIntoValue<T> for SingleQuery<T>
where
	T: SyncBounds + FromStr,
	T::Err: SyncBounds,
{
	fn into_query_value(value: T) -> Self::Value {
		Loadable::Loaded(value)
	}
}

impl<T> QueryIntoValue<Result<T, T::Err>> for SingleQuery<T>
where
	T: SyncBounds + FromStr,
	T::Err: SyncBounds,
{
	fn into_query_value(value: Result<T, T::Err>) -> Self::Value {
		match value {
			Ok(value) => Loadable::Loaded(value),
			Err(err) => Loadable::Err(err),
		}
	}
}

impl<T> QueryWrite<&'_ Loadable<T, T::Err>> for SingleQuery<T>
where
	T: SyncBounds + FromStr + ToString,
	T::Err: SyncBounds + StdError,
{
	fn write(&self, new_value: &Loadable<T, T::Err>) {
		match new_value {
			Loadable::Empty => self.write(None),
			Loadable::Err(err) => tracing::warn!(?self.key, ?err, "Cannot assign an error to a query value"),
			Loadable::Loaded(new_value) => self.write(Some(new_value)),
		}
	}
}

impl<T: FromStr<Err: StdError> + ToString> QueryWrite<Option<&'_ T>> for SingleQuery<T> {
	fn write(&self, new_value: Option<&T>) {
		// Update our queries memo manually and prevent it from being added
		let _suppress_queries = self.queries.suppress();
		match &new_value {
			Some(new_value) => self.queries.update_no_run(vec![new_value.to_string()]),
			None => self.queries.update_no_run(vec![]),
		}

		let location = self.ctx.store().expect_cloned::<LocationSignal>();
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

			// If it's our key, check what we should do
			if let Some(new_value) = new_value {
				queries.push((self.key.to_string(), new_value.to_string()));
				added_query = true;
			}
		}

		// If we haven't added ours yet by now, add it at the end
		if !added_query && let Some(new_value) = new_value {
			queries.push((self.key.to_string(), new_value.to_string()));
		}

		location.query_pairs_mut().clear().extend_pairs(queries);
	}
}
