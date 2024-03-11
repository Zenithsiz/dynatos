//! Query array signal

// Imports
use {
	crate::Location,
	core::{
		error::Error as StdError,
		mem,
		ops::{Deref, DerefMut},
		str::FromStr,
	},
	dynatos_reactive::{
		signal,
		Effect,
		Signal,
		SignalBorrow,
		SignalBorrowMut,
		SignalGetCloned,
		SignalReplace,
		SignalSet,
		SignalUpdate,
		SignalWith,
	},
	std::rc::Rc,
};

/// Query signal
#[derive(Clone, Debug)]
pub struct QueryArraySignal<T> {
	/// Key
	key: Rc<str>,

	/// Inner value
	inner: Signal<Vec<T>>,

	/// Update effect.
	update_effect: Effect<dyn Fn()>,
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
					.filter_map(|value| match value.parse::<T>() {
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

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T>(signal::BorrowRef<'a, Vec<T>>);

impl<'a, T> Deref for BorrowRef<'a, T> {
	type Target = [T];

	fn deref(&self) -> &Self::Target {
		self.0.as_slice()
	}
}

impl<T: 'static> SignalBorrow for QueryArraySignal<T> {
	type Ref<'a> = BorrowRef<'a, T>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		BorrowRef(self.inner.borrow())
	}
}

impl<T: 'static> SignalWith for QueryArraySignal<T> {
	type Value<'a> = &'a [T];

	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(&value)
	}
}

impl<T> SignalReplace<Vec<T>> for QueryArraySignal<T>
where
	T: ToString + 'static,
{
	fn replace(&self, new_value: Vec<T>) -> Vec<T> {
		mem::replace(&mut self.borrow_mut(), new_value)
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a, T>
where
	T: ToString,
{
	/// Value
	value: signal::BorrowRefMut<'a, Vec<T>>,

	/// Signal
	signal: &'a QueryArraySignal<T>,
}

impl<'a, T> Deref for BorrowRefMut<'a, T>
where
	T: ToString,
{
	type Target = Vec<T>;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<'a, T> DerefMut for BorrowRefMut<'a, T>
where
	T: ToString,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.value
	}
}

impl<'a, T> Drop for BorrowRefMut<'a, T>
where
	T: ToString,
{
	fn drop(&mut self) {
		// Update the location
		// Note: We suppress the update, given that it won't change anything,
		//       as we already have the latest value.
		// TODO: Force an update anyway just to ensure some consistency with `FromStr` + `ToString`?
		self.signal.update_effect.suppressed(|| {
			dynatos_context::with_expect::<Location, _, _>(|location| {
				let mut location = location.borrow_mut();
				let mut queries = location
					.query_pairs()
					.into_owned()
					.filter(|(key, _)| *key != *self.signal.key)
					.collect::<Vec<_>>();
				for value in &*self.value {
					let value = value.to_string();
					queries.push(((*self.signal.key).to_owned(), value));
				}
				location.query_pairs_mut().clear().extend_pairs(queries);
			});
		});
	}
}

impl<T> SignalBorrowMut for QueryArraySignal<T>
where
	T: ToString + 'static,
{
	type RefMut<'a> = BorrowRefMut<'a, T>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.inner.borrow_mut();
		BorrowRefMut { value, signal: self }
	}
}

impl<T> SignalUpdate for QueryArraySignal<T>
where
	T: ToString + 'static,
{
	type Value<'a> = &'a mut Vec<T>;

	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut value = self.borrow_mut();
		f(&mut *value)
	}
}
