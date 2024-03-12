//! Query signal

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
	std::{collections::HashMap, rc::Rc},
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

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T>(signal::BorrowRef<'a, Option<T>>);

impl<'a, T> Deref for BorrowRef<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Inner query value was missing")
	}
}

impl<T: 'static> SignalBorrow for QuerySignal<T> {
	type Ref<'a> = Option<BorrowRef<'a, T>>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		let borrow = self.inner.borrow();
		borrow.is_some().then(|| BorrowRef(borrow))
	}
}

impl<T: 'static> SignalWith for QuerySignal<T> {
	type Value<'a> = Option<&'a T>;

	#[track_caller]
	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let value = self.borrow();
		f(value.as_deref())
	}
}

impl<T> SignalReplace<Option<T>> for QuerySignal<T>
where
	T: ToString + 'static,
{
	#[track_caller]
	fn replace(&self, new_value: Option<T>) -> Option<T> {
		mem::replace(&mut self.borrow_mut(), new_value)
	}
}

/// Updates the location on `Drop`
// Note: We need this wrapper because `BorrowRefMut::value` must
//       already be dropped when we update the location, which we
//       can't do if we implement `Drop` on `BorrowRefMut`.
#[derive(Debug)]
struct UpdateLocationOnDrop<'a, T: ToString + 'static>(pub &'a QuerySignal<T>);

impl<'a, T> Drop for UpdateLocationOnDrop<'a, T>
where
	T: ToString + 'static,
{
	fn drop(&mut self) {
		// Update the location
		// Note: We suppress the update, given that it won't change anything,
		//       as we already have the latest value.
		// TODO: Force an update anyway just to ensure some consistency with `FromStr` + `ToString`?
		self.0.update_effect.suppressed(|| {
			dynatos_context::with_expect::<Location, _, _>(|location| {
				let mut location = location.borrow_mut();
				let mut queries = location.query_pairs().into_owned().collect::<HashMap<_, _>>();
				match self.0.borrow().as_deref().map(T::to_string) {
					Some(value) => queries.insert((*self.0.key).to_owned(), value),
					None => queries.remove(&*self.0.key),
				};

				location.query_pairs_mut().clear().extend_pairs(queries);
			});
		});
	}
}

/// Reference type for [`SignalBorrowMut`] impl
#[derive(Debug)]
pub struct BorrowRefMut<'a, T>
where
	T: ToString + 'static,
{
	/// Value
	value: signal::BorrowRefMut<'a, Option<T>>,

	/// Update location on drop
	// Note: Must be dropped *after* `value`.
	_update_location_on_drop: UpdateLocationOnDrop<'a, T>,
}

impl<'a, T> Deref for BorrowRefMut<'a, T>
where
	T: ToString,
{
	type Target = Option<T>;

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

impl<T> SignalBorrowMut for QuerySignal<T>
where
	T: ToString + 'static,
{
	type RefMut<'a> = BorrowRefMut<'a, T>
	where
		Self: 'a;

	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.inner.borrow_mut();
		BorrowRefMut {
			value,
			_update_location_on_drop: UpdateLocationOnDrop(self),
		}
	}
}

impl<T> SignalUpdate for QuerySignal<T>
where
	T: ToString + 'static,
{
	type Value<'a> = &'a mut Option<T>;

	#[track_caller]
	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut value = self.borrow_mut();
		f(&mut *value)
	}
}
