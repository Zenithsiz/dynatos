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
	dynatos_reactive::{signal, Effect, Memo, Signal, SignalBorrow, SignalBorrowMut, SignalReplace, SignalSet},
	std::rc::Rc,
	zutil_cloned::cloned,
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
	#[track_caller]
	pub fn new<K>(key: K) -> Self
	where
		T: FromStr + 'static,
		T::Err: StdError + Send + Sync + 'static,
		K: Into<Rc<str>>,
	{
		// Get the query values
		let key = key.into();
		let query_values = Memo::new({
			let key = Rc::clone(&key);
			move || {
				dynatos_context::with_expect::<Location, _, _>(|location| {
					location
						.borrow()
						.query_pairs()
						.filter_map(|(query, value)| (query == *key).then_some(value.into_owned()))
						.collect::<Vec<_>>()
				})
			}
		});

		let inner = Signal::new(vec![]);
		#[cloned(inner, key)]
		let update = Effect::new(move || {
			let values = query_values
				.borrow()
				.iter()
				.filter_map(|query_value| match query_value.parse::<T>() {
					Ok(value) => Some(value),
					Err(err) => {
						tracing::warn!(?key, value=?query_value, ?err, "Unable to parse query");
						None
					},
				})
				.collect();

			// Then set it
			inner.set(values);
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

impl<T> Deref for BorrowRef<'_, T> {
	type Target = [T];

	fn deref(&self) -> &Self::Target {
		self.0.as_slice()
	}
}

impl<T: 'static> SignalBorrow for QueryArraySignal<T> {
	type Ref<'a>
		= BorrowRef<'a, T>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		BorrowRef(self.inner.borrow())
	}

	#[track_caller]
	fn borrow_raw(&self) -> Self::Ref<'_> {
		BorrowRef(self.inner.borrow_raw())
	}
}

impl<T> SignalReplace<Vec<T>> for QueryArraySignal<T>
where
	T: ToString + 'static,
{
	type Value = Vec<T>;

	#[track_caller]
	fn replace(&self, new_value: Vec<T>) -> Self::Value {
		mem::replace(&mut self.borrow_mut(), new_value)
	}
}

/// Updates the location on `Drop`
// Note: We need this wrapper because `BorrowRefMut::value` must
//       already be dropped when we update the location, which we
//       can't do if we implement `Drop` on `BorrowRefMut`.
#[derive(Debug)]
struct UpdateLocationOnDrop<'a, T: ToString + 'static>(pub &'a QueryArraySignal<T>);

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
				let mut queries = location
					.query_pairs()
					.into_owned()
					.filter(|(key, _)| *key != *self.0.key)
					.collect::<Vec<_>>();

				// Note: We can't use a normal `borrow`, because that'd add us as a dependency to any
				//       running effects, but that might cause loops since updating the location would
				//       update us as well.
				for value in &*self.0.inner.borrow_raw() {
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
	value: signal::BorrowRefMut<'a, Vec<T>>,

	/// Update location on drop
	// Note: Must be dropped *after* `value`.
	_update_location_on_drop: Option<UpdateLocationOnDrop<'a, T>>,
}

impl<T> Deref for BorrowRefMut<'_, T>
where
	T: ToString,
{
	type Target = Vec<T>;

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


impl<T> SignalBorrowMut for QueryArraySignal<T>
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

impl<T> signal::SignalSetDefaultImpl for QueryArraySignal<T> {}
impl<T> signal::SignalWithDefaultImpl for QueryArraySignal<T> {}
impl<T> signal::SignalUpdateDefaultImpl for QueryArraySignal<T> {}
