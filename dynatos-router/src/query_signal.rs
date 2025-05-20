//! Query signal

// Imports
use {
	crate::Location,
	core::{
		fmt,
		mem,
		ops::{Deref, DerefMut},
		str::FromStr,
	},
	dynatos_loadable::Loadable,
	dynatos_reactive::{signal, Effect, Memo, Signal, SignalBorrow, SignalBorrowMut, SignalReplace, SignalSet},
	std::rc::Rc,
	zutil_cloned::cloned,
};

/// Query signal
pub struct QuerySignal<T, E = <T as FromStr>::Err> {
	/// Key
	key: Rc<str>,

	/// Inner value
	inner: Signal<Loadable<T, E>>,

	/// Update effect.
	update_effect: Effect<dyn Fn()>,
}

impl<T, E> QuerySignal<T, E> {
	/// Creates a new query signal for `key`.
	///
	/// Expects a context of type [`Location`](crate::Location).
	#[track_caller]
	pub fn new<K>(key: K) -> Self
	where
		T: FromStr + 'static,
		E: From<T::Err> + 'static,
		K: Into<Rc<str>>,
	{
		// Get the query value
		let key = key.into();
		#[cloned(key)]
		let query_value = Memo::new(move || {
			dynatos_context::with_expect::<Location, _, _>(|location| {
				location
					.borrow()
					.query_pairs()
					.find_map(|(query, value)| (query == *key).then_some(value.into_owned()))
			})
		});

		let inner = Signal::new(Loadable::Empty);
		#[cloned(inner)]
		let update = Effect::new(move || {
			let value = match query_value.borrow().as_ref() {
				Some(value) => match value.parse::<T>() {
					Ok(value) => Loadable::Loaded(value),
					Err(err) => Loadable::Err(err.into()),
				},
				None => Loadable::Empty,
			};

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

impl<T, E> Clone for QuerySignal<T, E> {
	fn clone(&self) -> Self {
		Self {
			key:           Rc::clone(&self.key),
			inner:         self.inner.clone(),
			update_effect: self.update_effect.clone(),
		}
	}
}

impl<T, E> fmt::Debug for QuerySignal<T, E>
where
	T: fmt::Debug,
	E: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("QuerySignal")
			.field("key", &self.key)
			.field("inner", &self.inner)
			.field("update_effect", &self.update_effect)
			.finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T, E = <T as FromStr>::Err>(signal::BorrowRef<'a, Loadable<T, E>>);

impl<T, E> Deref for BorrowRef<'_, T, E> {
	type Target = Loadable<T, E>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T, E> SignalBorrow for QuerySignal<T, E>
where
	T: 'static,
	E: 'static,
{
	type Ref<'a>
		= BorrowRef<'a, T, E>
	where
		Self: 'a;

	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_> {
		BorrowRef(self.inner.borrow())
	}

	fn borrow_raw(&self) -> Self::Ref<'_> {
		BorrowRef(self.inner.borrow_raw())
	}
}

impl<T, E> SignalReplace<Loadable<T, E>> for QuerySignal<T, E>
where
	T: ToString + 'static,
	E: 'static,
{
	type Value = Loadable<T, E>;

	#[track_caller]
	fn replace(&self, new_value: Loadable<T, E>) -> Self::Value {
		mem::replace(&mut self.borrow_mut(), new_value)
	}

	#[track_caller]
	fn replace_raw(&self, new_value: Loadable<T, E>) -> Self::Value {
		mem::replace(&mut self.borrow_mut_raw(), new_value)
	}
}

/// Updates the location on `Drop`
// Note: We need this wrapper because `BorrowRefMut::value` must
//       already be dropped when we update the location, which we
//       can't do if we implement `Drop` on `BorrowRefMut`.
struct UpdateLocationOnDrop<'a, T: ToString + 'static, E: 'static = <T as FromStr>::Err>(pub &'a QuerySignal<T, E>);

impl<'a, T, E> fmt::Debug for UpdateLocationOnDrop<'a, T, E>
where
	T: ToString + 'static + fmt::Debug,
	E: 'static,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("UpdateLocationOnDrop").finish_non_exhaustive()
	}
}

impl<T, E> Drop for UpdateLocationOnDrop<'_, T, E>
where
	T: ToString + 'static,
	E: 'static,
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
							Loadable::Loaded(value) => {
								added_query = true;
								Some(((*self.0.key).to_owned(), value.to_string()))
							},
							Loadable::Err(_) => {
								tracing::warn!(key=?self.0.key, "Cannot assign an error to a query value");
								None
							},
							Loadable::Empty => None,
						}
					})
					.collect::<Vec<_>>();

				// If we haven't added ours yet by now, add it at the end
				if !added_query {
					match &*self.0.inner.borrow_raw() {
						Loadable::Loaded(value) => queries.push(((*self.0.key).to_owned(), value.to_string())),
						Loadable::Err(_) => tracing::warn!(key=?self.0.key, "Cannot assign an error to a query value"),
						Loadable::Empty => (),
					}
				}

				location.query_pairs_mut().clear().extend_pairs(queries);
			});
		});
	}
}

/// Reference type for [`SignalBorrowMut`] impl
pub struct BorrowRefMut<'a, T, E = <T as FromStr>::Err>
where
	T: ToString + 'static,
	E: 'static,
{
	/// Value
	value: signal::BorrowRefMut<'a, Loadable<T, E>>,

	/// Update location on drop
	// Note: Must be dropped *after* `value`.
	update_location_on_drop: Option<UpdateLocationOnDrop<'a, T, E>>,
}

impl<'a, T, E> fmt::Debug for BorrowRefMut<'a, T, E>
where
	T: ToString + fmt::Debug + 'static,
	E: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("BorrowRefMut")
			.field("value", &self.value)
			.field("update_location_on_drop", &self.update_location_on_drop)
			.finish()
	}
}

impl<T, E> Deref for BorrowRefMut<'_, T, E>
where
	T: ToString,
{
	type Target = Loadable<T, E>;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<T, E> DerefMut for BorrowRefMut<'_, T, E>
where
	T: ToString,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.value
	}
}

impl<T, E> SignalBorrowMut for QuerySignal<T, E>
where
	T: ToString + 'static,
	E: 'static,
{
	type RefMut<'a>
		= BorrowRefMut<'a, T, E>
	where
		Self: 'a;

	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.inner.borrow_mut();
		BorrowRefMut {
			value,
			update_location_on_drop: Some(UpdateLocationOnDrop(self)),
		}
	}

	#[track_caller]
	fn borrow_mut_raw(&self) -> Self::RefMut<'_> {
		// TODO: Should we be updating the location on drop?
		let value = self.inner.borrow_mut_raw();
		BorrowRefMut {
			value,
			update_location_on_drop: None,
		}
	}
}

impl<T, E> signal::SignalSetDefaultImpl for QuerySignal<T, E> {}
impl<T, E> signal::SignalWithDefaultImpl for QuerySignal<T, E> {}
impl<T, E> signal::SignalUpdateDefaultImpl for QuerySignal<T, E> {}
