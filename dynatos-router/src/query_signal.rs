//! Query signal

// Modules
pub mod multi_query;
pub mod single_query;

// Exports
pub use self::{multi_query::MultiQuery, single_query::SingleQuery};

// Imports
use {
	crate::Location,
	core::{
		fmt,
		mem,
		ops::{Deref, DerefMut},
	},
	dynatos_reactive::{
		signal,
		Effect,
		EffectRun,
		Memo,
		Signal,
		SignalBorrow,
		SignalBorrowMut,
		SignalReplace,
		SignalSet,
	},
	std::rc::Rc,
	zutil_cloned::cloned,
};

/// Query signal.
///
/// # Bind
/// When creating a query signal, it will be bound to the current
/// url's path, and if the path changes, the query signal won't attempt
/// to update the value.
pub struct QuerySignal<T: QueryParse + 'static> {
	/// Query
	query: Rc<T>,

	/// Bound url path for this query.
	// TODO: Should this be optional so the user can opt out of
	//       binding a unique path against the query signal?
	location_path: Rc<str>,

	/// Inner value
	inner: Signal<Option<T::Value>>,

	/// Update effect.
	update_effect: Effect<UpdateEffect<T>>,
}

impl<T: QueryParse> QuerySignal<T> {
	/// Creates a new query signal with `query`.
	#[track_caller]
	#[define_opaque(UpdateEffect)]
	pub fn new(query: T) -> Self
	where
		T: 'static,
		T::Value: 'static,
	{
		let query = Rc::new(query);

		// Note: This access must be raw to ensure that the query signal itself doesn't
		//       change whenever the location changes, and only it's value does.
		let location = dynatos_context::expect_cloned::<Location>();
		let location_path = Rc::<str>::from(location.borrow_raw().path());

		let inner = Signal::new(None);
		#[cloned(query, location_path, inner)]
		let update = Effect::new(move || {
			// If the location changes, don't update.
			// Note: If this is the correct location, we don't want to add the location
			//       as a dependency, because otherwise we'll be re-parsing all queries
			//       each time the location changes and not just when `T::parse` changes.
			if *location.borrow_raw().path() != *location_path {
				_ = location.borrow();
				return;
			}

			let value = query.parse();
			inner.set(value);
		});

		Self {
			query,
			location_path,
			inner,
			update_effect: update,
		}
	}

	/// Returns the query of this signal
	#[must_use]
	pub fn query(&self) -> &T {
		&self.query
	}
}

type UpdateEffect<T: QueryParse + 'static> = impl EffectRun;

impl<T: QueryParse> Clone for QuerySignal<T> {
	fn clone(&self) -> Self {
		Self {
			query:         Rc::clone(&self.query),
			location_path: Rc::clone(&self.location_path),
			inner:         self.inner.clone(),
			update_effect: self.update_effect.clone(),
		}
	}
}

impl<T> fmt::Debug for QuerySignal<T>
where
	T: QueryParse + 'static + fmt::Debug,
	T::Value: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("QuerySignal")
			.field("query", &self.query)
			.field("location_path", &self.location_path)
			.field("inner", &self.inner)
			.field("update_effect", &self.update_effect)
			.finish()
	}
}

/// Reference type for [`SignalBorrow`] impl
#[derive(Debug)]
pub struct BorrowRef<'a, T: QueryParse>(signal::BorrowRef<'a, Option<T::Value>>);

impl<T: QueryParse> Deref for BorrowRef<'_, T> {
	type Target = T::Value;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().expect("Should have value")
	}
}

impl<T> SignalBorrow for QuerySignal<T>
where
	T: QueryParse + 'static,
	T::Value: 'static,
{
	type Ref<'a>
		= BorrowRef<'a, T>
	where
		Self: 'a;

	fn borrow(&self) -> Self::Ref<'_> {
		BorrowRef(self.inner.borrow())
	}

	fn borrow_raw(&self) -> Self::Ref<'_> {
		BorrowRef(self.inner.borrow_raw())
	}
}

impl<T> SignalReplace<T::Value> for QuerySignal<T>
where
	T: QueryParse + QueryWriteValue + 'static,
	T::Value: 'static,
{
	type Value = T::Value;

	fn replace(&self, new_value: T::Value) -> Self::Value {
		mem::replace(&mut *self.borrow_mut(), new_value)
	}

	fn replace_raw(&self, new_value: T::Value) -> Self::Value {
		mem::replace(&mut *self.borrow_mut_raw(), new_value)
	}
}

impl<T, U> SignalSet<U> for QuerySignal<T>
where
	T: QueryParse + QueryIntoValue<U> + QueryWriteValue + 'static,
	T::Value: 'static,
{
	fn set(&self, new_value: U) {
		*self.borrow_mut() = T::into_query_value(new_value);
	}

	fn set_raw(&self, new_value: U) {
		*self.borrow_mut_raw() = T::into_query_value(new_value);
	}
}

/// Reference type for [`SignalBorrowMut`] impl
pub struct BorrowRefMut<'a, T>
where
	T: QueryParse + QueryWriteValue + 'static,
	T::Value: 'static,
{
	/// Value
	value: signal::BorrowRefMut<'a, Option<T::Value>>,

	/// Query signal
	signal: &'a QuerySignal<T>,

	/// Whether to write the query on drop.
	write_query_on_drop: bool,
}

impl<T> fmt::Debug for BorrowRefMut<'_, T>
where
	T: QueryParse + QueryWriteValue + 'static,
	T::Value: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("BorrowRefMut")
			.field("value", &self.value)
			.field("update_location_on_drop", &self.write_query_on_drop)
			.finish()
	}
}

impl<T> Deref for BorrowRefMut<'_, T>
where
	T: QueryParse + QueryWriteValue + 'static,
{
	type Target = T::Value;

	fn deref(&self) -> &Self::Target {
		self.value.as_ref().expect("Should have value")
	}
}

impl<T> DerefMut for BorrowRefMut<'_, T>
where
	T: QueryParse + QueryWriteValue + 'static,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.value.as_mut().expect("Should have value")
	}
}

impl<T: QueryWriteValue> Drop for BorrowRefMut<'_, T> {
	fn drop(&mut self) {
		// Write the signal back
		if self.write_query_on_drop {
			// Note: We suppress the update, given that it won't change anything,
			//       as we already have the latest value.
			// TODO: Force an update anyway just to ensure some consistency with `FromStr` + `ToString`?
			let _suppressed = self.signal.update_effect.suppress();

			// Note: It's fine to do this while the value is still alive because
			//       the run queue ensures we won't run any effects until we're dropped.
			// TODO: If we're being borrowed raw, this might bypass the run queue, and end
			//       up writing while our borrow still exists?
			self.signal.query.write(&*self);
		}
	}
}

impl<T> SignalBorrowMut for QuerySignal<T>
where
	T: QueryParse + QueryWriteValue + 'static,
	T::Value: 'static,
{
	type RefMut<'a>
		= BorrowRefMut<'a, T>
	where
		Self: 'a;

	fn borrow_mut(&self) -> Self::RefMut<'_> {
		let value = self.inner.borrow_mut();
		BorrowRefMut {
			value,
			signal: self,
			write_query_on_drop: true,
		}
	}

	fn borrow_mut_raw(&self) -> Self::RefMut<'_> {
		// TODO: Should we be updating the location on drop?
		let value = self.inner.borrow_mut_raw();
		BorrowRefMut {
			value,
			signal: self,
			write_query_on_drop: false,
		}
	}
}

// Note: We want a broader set impl to allow setting `T`s in `Loadable<T, E>`s.
impl<T: QueryParse + 'static> !signal::SignalSetDefaultImpl for QuerySignal<T> {}
impl<T: QueryParse + 'static> signal::SignalGetDefaultImpl for QuerySignal<T> {}
impl<T: QueryParse + 'static> signal::SignalGetClonedDefaultImpl for QuerySignal<T> {}
impl<T: QueryParse + 'static> signal::SignalWithDefaultImpl for QuerySignal<T> {}
impl<T: QueryParse + 'static> signal::SignalUpdateDefaultImpl for QuerySignal<T> {}


/// Query parse
pub trait QueryParse {
	/// Value
	type Value;

	/// Parses the value from the query
	fn parse(&self) -> Self::Value;
}


/// Query into value
pub trait QueryIntoValue<T>: QueryParse {
	/// Converts this type into the value type of a query
	fn into_query_value(value: T) -> Self::Value;
}

impl<Q: QueryParse> QueryIntoValue<Q::Value> for Q {
	fn into_query_value(value: Q::Value) -> Self::Value {
		value
	}
}

/// Query write
pub trait QueryWrite<T> {
	/// Writes the value back into the query
	#[track_caller]
	fn write(&self, new_value: T);
}

/// Alias for a query that can write a reference to it's own value type
pub trait QueryWriteValue = QueryParse + for<'a> QueryWrite<&'a <Self as QueryParse>::Value>;

type QueriesFn = impl Fn() -> Vec<String>;

#[define_opaque(QueriesFn)]
fn queries_memo(key: Rc<str>) -> Memo<Vec<String>, QueriesFn> {
	Memo::new(move || {
		dynatos_context::with_expect::<Location, _, _>(|location| {
			location
				.borrow()
				.query_pairs()
				.filter_map(|(query, value)| (query == *key).then_some(value.into_owned()))
				.collect::<Vec<_>>()
		})
	})
}
