//! Signal operators

/// Types which may be copied by [`SignalGet`]
pub trait SignalGetCopy<T>: Sized {
	fn copy_value(self) -> T;
}

impl<T: Copy> SignalGetCopy<T> for &'_ T {
	fn copy_value(self) -> T {
		*self
	}
}
impl<T: Copy> SignalGetCopy<Option<T>> for Option<&'_ T> {
	fn copy_value(self) -> Option<T> {
		self.copied()
	}
}

/// Signal get
pub trait SignalGet<T> {
	/// Gets the signal value, by copying it.
	fn get(&self) -> T;
}

impl<S, T> SignalGet<T> for S
where
	S: SignalWith,
	for<'a> S::Value<'a>: SignalGetCopy<T>,
{
	fn get(&self) -> T {
		self.with(|value| value.copy_value())
	}
}

/// Types which may be cloned by [`SignalGetCloned`]
pub trait SignalGetClone<T>: Sized {
	fn clone_value(self) -> T;
}

impl<T: Clone> SignalGetClone<T> for &'_ T {
	fn clone_value(self) -> T {
		self.clone()
	}
}
impl<T: Clone> SignalGetClone<Option<T>> for Option<&'_ T> {
	fn clone_value(self) -> Option<T> {
		self.cloned()
	}
}

/// Signal cloned
pub trait SignalGetCloned<T> {
	/// Gets the signal value, by cloning it.
	fn get_cloned(&self) -> T;
}

impl<S, T> SignalGetCloned<T> for S
where
	S: SignalWith,
	for<'a> S::Value<'a>: SignalGetClone<T>,
{
	fn get_cloned(&self) -> T {
		self.with(|value| value.clone_value())
	}
}

/// Signal with
pub trait SignalWith {
	/// Value type
	type Value<'a>: ?Sized;

	/// Uses the signal value
	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O;
}

/// Types which may be set by [`SignalSet`]
pub trait SignalSetWith<T>: Sized {
	fn set(self, new_value: T);
}

impl<T> SignalSetWith<T> for &'_ mut T {
	fn set(self, new_value: T) {
		*self = new_value;
	}
}
impl<T> SignalSetWith<T> for &'_ mut Option<T> {
	fn set(self, new_value: T) {
		*self = Some(new_value);
	}
}

/// Signal set
pub trait SignalSet<Value> {
	/// Sets the signal value
	fn set(&self, new_value: Value);
}

impl<S, T> SignalSet<T> for S
where
	S: SignalUpdate,
	for<'a> S::Value<'a>: SignalSetWith<T>,
{
	fn set(&self, new_value: T) {
		self.update(|value| SignalSetWith::set(value, new_value));
	}
}

/// Signal replace
pub trait SignalReplace<Value> {
	/// Replaces the signal value, returning the previous value
	fn replace(&self, new_value: Value) -> Value;
}

/// Signal update
pub trait SignalUpdate {
	/// Value type
	type Value<'a>: ?Sized;

	/// Updates the signal value
	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O;
}
