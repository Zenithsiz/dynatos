//! Object

// Imports
use {
	crate::JsValue,
	dynatos_inheritance::{Downcast, Value},
	std::{collections::HashMap, sync::nonpoison::Mutex},
};

dynatos_inheritance::value! {
	pub struct Object(): Send + Sync + Debug + Default {
		props: Mutex<HashMap<String, JsValue>>,
	}
	impl Self {}
}

impl Object {
	pub fn get_prop_inner<T: Value>(&self, prop: &str) -> Result<T, GetError> {
		let fields = self.fields().props.lock();
		let value = fields.get(prop).ok_or(GetError::Missing)?.clone();

		match value.try_into_object() {
			Ok(obj) => obj.downcast::<T>().map_err(|obj| GetError::WrongType(obj.into())),
			Err(value) => Err(GetError::WrongType(value)),
		}
	}

	pub fn set_prop_inner<T: Into<JsValue>>(&self, prop: &str, value: T) {
		self.fields().props.lock().insert(prop.to_owned(), value.into());
	}

	#[expect(clippy::must_use_candidate, reason = "User might just want to remove it")]
	pub fn remove_prop_inner(&self, prop: &str) -> Option<JsValue> {
		self.fields().props.lock().remove(prop)
	}
}

#[derive(Clone, Debug)]
pub enum GetError {
	/// Property was missing
	Missing,

	/// Property was the wrong type
	WrongType(JsValue),
}
