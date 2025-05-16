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
		Memo,
		Signal,
		SignalBorrow,
		SignalBorrowMut,
		SignalReplace,
		SignalSet,
		SignalWith,
	},
	std::rc::Rc,
	zutil_cloned::cloned,
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
	#[track_caller]
	pub fn new<K>(key: K) -> Self
	where
		T: FromStr + 'static,
		T::Err: StdError + Send + Sync + 'static,
		K: Into<Rc<str>>,
	{
		// Get the query value
		let key = key.into();
		let query_value = Memo::new({
			let key = Rc::clone(&key);
			move || {
				dynatos_context::with_expect::<Location, _, _>(|location| {
					location
						.borrow()
						.query_pairs()
						.find_map(|(query, value)| (query == *key).then_some(value.into_owned()))
				})
			}
		});

		let inner = Signal::new(None);
		#[cloned(inner, key)]
		let update = Effect::new(move || {
			let value = query_value
				.borrow()
				.as_ref()
				.and_then(|query_value| match query_value.parse::<T>() {
					Ok(value) => Some(value),
					Err(err) => {
						tracing::warn!(?key, value=?query_value, ?err, "Unable to parse query");
						None
					},
				});

			// Then set it
			inner.set(value);
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

impl<T> Deref for BorrowRef<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Inner query value was missing")
	}
}

impl<T: 'static> SignalBorrow for QuerySignal<T> {
	type Ref<'a>
		= Option<BorrowRef<'a, T>>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		let borrow = self.inner.borrow();
		borrow.is_some().then(|| BorrowRef(borrow))
	}

	fn borrow_raw(&self) -> Self::Ref<'_> {
		let borrow = self.inner.borrow_raw();
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
	type Value = Option<T>;

	#[track_caller]
	fn replace(&self, new_value: Option<T>) -> Self::Value {
		mem::replace(&mut self.borrow_mut(), new_value)
	}
}

/// Updates the location on `Drop`
// Note: We need this wrapper because `BorrowRefMut::value` must
//       already be dropped when we update the location, which we
//       can't do if we implement `Drop` on `BorrowRefMut`.
#[derive(Debug)]
struct UpdateLocationOnDrop<'a, T: ToString + 'static>(pub &'a QuerySignal<T>);

impl<T> Drop for UpdateLocationOnDrop<'_, T>
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
				let mut added_query = false;
				let mut queries = location
					.query_pairs()
					.into_owned()
					.filter_map(|(key, value)| {
						// If it's another key, keep it
						if key != *self.0.key {
							return Some((key, value));
						}

						// If we already added our query, this is a duplicate, so skip it
						if added_query {
							return None;
						}

						// If it's our key, check what we should do
						match &*self.0.inner.borrow_raw() {
							Some(value) => {
								added_query = true;
								Some(((*self.0.key).to_owned(), value.to_string()))
							},
							None => None,
						}
					})
					.collect::<Vec<_>>();

				// If we haven't added ours yet by now, add it at the end
				if !added_query && let Some(value) = &*self.0.inner.borrow_raw() {
					queries.push(((*self.0.key).to_owned(), value.to_string()));
				}

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
	_update_location_on_drop: Option<UpdateLocationOnDrop<'a, T>>,
}

impl<T> Deref for BorrowRefMut<'_, T>
where
	T: ToString,
{
	type Target = Option<T>;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<T> DerefMut for BorrowRefMut<'_, T>
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
	type RefMut<'a>
		= BorrowRefMut<'a, T>
	where
		Self: 'a;

	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.inner.borrow_mut();
		BorrowRefMut {
			value,
			_update_location_on_drop: Some(UpdateLocationOnDrop(self)),
		}
	}

	#[track_caller]
	fn borrow_mut_raw(&self) -> Self::RefMut<'_> {
		// TODO: Should we be updating the location on drop?
		let value = self.inner.borrow_mut_raw();
		BorrowRefMut {
			value,
			_update_location_on_drop: None,
		}
	}
}

impl<T> signal::SignalSetDefaultImpl for QuerySignal<T> {}

// Note: We want to return an `Option<&T>`, so we can't use the default impl
// TODO: Should we just return an `&Option<T>` instead? That is a big API promise
//       due to requiring us to store an `Option<T>`, but the `SignalUpdate` impl
//       already exposes an `&mut Option<T>`, so maybe that's fine?
impl<T> !signal::SignalWithDefaultImpl for QuerySignal<T> {}

// Note: Unlike `SignalWith`, we return an `&mut Option<T>` instead of `Option<&mut T>`,
//       so the default impl is fine
impl<T> signal::SignalUpdateDefaultImpl for QuerySignal<T> {}
