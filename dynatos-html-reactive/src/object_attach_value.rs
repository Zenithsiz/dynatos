//! Object value attaching

// Imports
use {
	core::any::Any,
	dynatos_html::{ObjectGet, ObjectSetProp},
	wasm_bindgen::prelude::wasm_bindgen,
};

/// Extension trait to add a value to an object
#[extend::ext(name = ObjectAttachValue)]
pub impl js_sys::Object {
	/// Attaches a value to this object
	fn attach_value<T>(&self, value: T)
	where
		T: 'static,
	{
		// Get the values array, or create it, if it doesn't exist
		// TODO: Use an static anonymous symbol?
		let prop_name = "__dynatos_values";
		let values = match self.get::<js_sys::Array>(prop_name) {
			Ok(values) => values,
			Err(dynatos_html::GetError::WrongType(err)) => panic!("Values aray was the wrong type: {err:?}"),
			Err(dynatos_html::GetError::Missing) => {
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
	O: AsRef<js_sys::Object>,
{
	/// Attaches a value to this object.
	///
	/// Returns the object, for chaining
	fn with_value<T>(self, value: T) -> Self
	where
		T: 'static,
	{
		self.as_ref().attach_value(value);
		self
	}
}

/// A wasm value
#[wasm_bindgen]
#[expect(dead_code, reason = "We just want to keep the field alive, not use it")]
struct WasmValue(Box<dyn Any>);
