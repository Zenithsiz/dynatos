//! Object value attaching

use {
	crate::{ObjectGet, ObjectSetProp},
	core::any::Any,
	dynatos_sync_types::SyncBounds,
	js_sys::Object,
};

/// Extension trait to add a value to an object
#[extend::ext(name = ObjectAttachValue)]
pub impl Object {
	/// Attaches a value to this object
	fn attach_value<T>(&self, value: T)
	where
		T: SyncBounds + 'static,
	{
		// TODO: Use an static anonymous symbol?
		let prop_name = "__dynatos_values";

		// Get the values array, or create it, if it doesn't exist
		let values = match self.get::<js_sys::Array>(prop_name) {
			Ok(values) => values,
			Err(crate::GetError::WrongType(err)) => panic!("Values array was the wrong type: {err:?}"),
			Err(crate::GetError::Missing) => {
				let values = js_sys::Array::new();
				self.set_prop(prop_name, &values);
				values
			},
		};

		// Then push the values
		let value = WasmValue(Box::new(value));
		values.push(&value.into());
	}
}

/// Extension trait to add a value to an object
#[extend::ext(name = ObjectWithValue)]
pub impl<O> O
where
	O: AsRef<Object>,
{
	/// Attaches a value to this object.
	///
	/// Returns the object, for chaining
	fn with_value<T>(self, value: T) -> Self
	where
		T: SyncBounds + 'static,
	{
		self.as_ref().attach_value(value);
		self
	}
}

/// A wasm value
#[wasm_bindgen::prelude::wasm_bindgen]
#[expect(dead_code, reason = "We just want to keep the field alive, not use it")]
struct WasmValue(Box<dyn Any>);
