//! Query signal

// Imports
use {
	crate::Location,
	dynatos_reactive::{Effect, Signal, SignalGetCloned, SignalReplace, SignalSet, SignalUpdate, SignalWith},
	std::{collections::HashMap, error::Error as StdError, mem, rc::Rc, str::FromStr},
};

/// Query signal
#[derive(Clone, Debug)]
pub struct QuerySignal<T> {
	/// Key
	key: Rc<str>,

	/// Inner value
	inner: Signal<Option<T>>,

	/// Update effect.
	update_effect: Effect<dyn Fn()>,
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
				let location = dynatos_context::with_expect::<Location, _, _>(|location| location.get_cloned());
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
}

impl<T> SignalWith for QuerySignal<T> {
	type Value = Option<T>;

	fn with<F, O>(&self, f: F) -> O
	where
		F: FnOnce(&Self::Value) -> O,
	{
		self.inner.with(|value| f(value))
	}
}

impl<T> SignalSet<Option<T>> for QuerySignal<T>
where
	T: ToString,
{
	fn set(&self, new_value: Option<T>) {
		self.update(|value| *value = new_value);
	}
}

impl<T> SignalSet<T> for QuerySignal<T>
where
	T: ToString,
{
	fn set(&self, new_value: T) {
		self.update(|value| *value = Some(new_value));
	}
}

impl<T> SignalReplace<Option<T>> for QuerySignal<T>
where
	T: ToString,
{
	fn replace(&self, new_value: Option<T>) -> Option<T> {
		self.update(|value| mem::replace(value, new_value))
	}
}

impl<T> SignalUpdate for QuerySignal<T>
where
	T: ToString,
{
	type Value = Option<T>;

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
					let mut queries = location.query_pairs().into_owned().collect::<HashMap<_, _>>();
					match self.inner.with(|value| value.as_ref().map(T::to_string)) {
						Some(value) => queries.insert((*self.key).to_owned(), value),
						None => queries.remove(&*self.key),
					};

					location.query_pairs_mut().clear().extend_pairs(queries);
				});
			})
		});

		output
	}
}
