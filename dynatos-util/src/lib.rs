//! Utilities for [`dynatos`]

// Features
#![feature(decl_macro)]

// Imports
use {
	std::fmt,
	wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue},
};

/// Extension trait to be able to use `.context` on `Result<T, JsValue>`.
#[extend::ext(name = JsResultContext)]
pub impl<T> Result<T, JsValue> {
	fn context<C>(self, context: C) -> Result<T, anyhow::Error>
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		self.map_err(|err| {
			let err = format!("{err:?}");
			let err = anyhow::Error::msg(err);
			err.context(context)
		})
	}
}

/// Extension trait to define a property on an object
#[extend::ext(name = ObjectDefineProperty)]
pub impl js_sys::Object {
	fn define_property<T>(&self, property: &str, value: T)
	where
		T: Into<JsValue>,
	{
		/// Object descriptor to use for `Object::define_property`.
		#[wasm_bindgen]
		pub struct ObjectDescriptor {
			value: JsValue,
		}

		#[wasm_bindgen]
		impl ObjectDescriptor {
			#[wasm_bindgen(getter)]
			pub fn value(&self) -> JsValue {
				self.value.clone()
			}
		}


		// Setup the descriptor
		let descriptor = ObjectDescriptor { value: value.into() };
		let descriptor = JsValue::from(descriptor);
		let descriptor = descriptor.dyn_into::<js_sys::Object>().expect("Should be an object");

		// Then define it.
		js_sys::Object::define_property(self, &JsValue::from_str(property), &descriptor);
	}
}
