//! Query signal

// Imports
use {
	crate::Location,
	dynatos_reactive::{Effect, Signal},
	std::{collections::HashMap, error::Error as StdError, rc::Rc, str::FromStr},
};

/// Query signal
#[derive(Clone)]
pub struct QuerySignal<T> {
	/// Key
	key: Rc<str>,

	/// Inner value
	inner: Signal<Option<T>>,

	/// Update effect.
	update_effect: Effect,
}

impl<T> QuerySignal<T> {
	/// Creates a new query signal for `key`.
	///
	/// Expects a context of type [`Location`](crate::Location).
	pub fn new<K>(key: K) -> Self
	where
		T: FromStr + 'static,
		T::Err: StdError + Send + Sync + 'static,
		K: Into<Rc<str>>,
	{
		let key = key.into();

		let inner = Signal::new(None);
		let update = Effect::new({
			let inner = inner.clone();
			let key = Rc::clone(&key);
			move || {
				// Get the location and find our query key, if any
				let location = dynatos_context::with_expect::<Location, _, _>(|location| location.get());
				let value = location
					.query_pairs()
					.find_map(|(query, value)| (query == *key).then_some(value))
					.and_then(|value| match value.parse::<T>() {
						Ok(value) => Some(value),
						Err(err) => {
							tracing::warn!(?key, ?value, ?err, "Unable to parse query");
							None
						},
					});

				inner.set(value);
			}
		});

		Self {
			key,
			inner,
			update_effect: update,
		}
	}

	/// Returns the query signal
	pub fn get(&self) -> Option<T>
	where
		T: Copy,
	{
		self.with(|value| value.copied())
	}

	/// Uses the query signal
	pub fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&T>) -> O,
	{
		self.inner.with(|value| f(value.as_ref()))
	}

	/// Sets the query signal
	pub fn set<V>(&self, new_value: V)
	where
		T: ToString,
		V: Into<Option<T>>,
	{
		self.update(|value| *value = new_value.into())
	}

	/// Updates the query signal
	pub fn update<F, O>(&self, f: F) -> O
	where
		T: ToString,
		F: FnOnce(&mut Option<T>) -> O,
	{
		// Update the value
		let output = self.inner.update(f);

		// Then update the location
		// Note: We suppress the update, given that it won't change anything,
		//       as we already have the latest value.
		// TODO: Force an update anyway just to ensure some consistency with `FromStr` + `ToString`?
		self.update_effect.suppressed(|| {
			dynatos_context::with_expect::<Location, _, _>(|location| {
				location
					.update(|location| {
						let mut queries = location.query_pairs().into_owned().collect::<HashMap<_, _>>();
						match self.inner.with(|value| value.as_ref().map(T::to_string)) {
							Some(value) => queries.insert((*self.key).to_owned(), value),
							None => queries.remove(&*self.key),
						};

						location.query_pairs_mut().clear().extend_pairs(queries);
					})
					.expect("Unable to update location");
			})
		});

		output
	}
}
