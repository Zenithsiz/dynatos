//! Query array signal

// Imports
use {
	crate::Location,
	dynatos_reactive::{Effect, Signal, SignalGetCloned, SignalReplace, SignalSet, SignalUpdate, SignalWith},
	std::{error::Error as StdError, mem, rc::Rc, str::FromStr},
};

/// Query signal
#[derive(Clone, Debug)]
pub struct QueryArraySignal<T> {
	/// Key
	key: Rc<str>,

	/// Inner value
	inner: Signal<Vec<T>>,

	/// Update effect.
	update_effect: Effect,
}

impl<T> QueryArraySignal<T> {
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

		let inner = Signal::new(vec![]);
		let update = Effect::new({
			let inner = inner.clone();
			let key = Rc::clone(&key);
			move || {
				// Get the location and find our query key, if any
				let location = dynatos_context::with_expect::<Location, _, _>(|location| location.get_cloned());
				let value = location
					.query_pairs()
					.filter_map(|(query, value)| (query == *key).then_some(value))
					.flat_map(|value| match value.parse::<T>() {
						Ok(value) => Some(value),
						Err(err) => {
							tracing::warn!(?key, ?value, ?err, "Unable to parse query");
							None
						},
					})
					.collect::<Vec<_>>();

				inner.set(value);
			}
		});

		Self {
			key,
			inner,
			update_effect: update,
		}
	}
}

impl<T> SignalWith for QueryArraySignal<T> {
	type Value = Vec<T>;

	fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Self::Value) -> O,
	{
		self.inner.with(|value| f(value))
	}
}

impl<T> SignalSet<Vec<T>> for QueryArraySignal<T>
where
	T: ToString,
{
	fn set(&self, new_value: Vec<T>) {
		self.update(|value| *value = new_value);
	}
}

impl<T> SignalReplace<Vec<T>> for QueryArraySignal<T>
where
	T: ToString,
{
	fn replace(&self, new_value: Vec<T>) -> Vec<T> {
		self.update(|value| mem::replace(value, new_value))
	}
}

impl<T> SignalUpdate for QueryArraySignal<T>
where
	T: ToString,
{
	type Value = Vec<T>;

	fn update<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&mut Self::Value) -> O,
	{
		// Update the value
		let output = self.inner.update(f);

		// Then update the location
		// Note: We suppress the update, given that it won't change anything,
		//       as we already have the latest value.
		// TODO: Force an update anyway just to ensure some consistency with `FromStr` + `ToString`?
		self.update_effect.suppressed(|| {
			dynatos_context::with_expect::<Location, _, _>(|location| {
				location.update(|location| {
					let mut queries = location
						.query_pairs()
						.into_owned()
						.filter(|(key, _)| *key != *self.key)
						.collect::<Vec<_>>();
					self.inner.with(|values| {
						for value in values {
							let value = value.to_string();
							queries.push(((*self.key).to_owned(), value));
						}
					});
					location.query_pairs_mut().clear().extend_pairs(queries);
				});
			})
		});

		output
	}
}
