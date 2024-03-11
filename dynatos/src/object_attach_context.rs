//! Object context attaching

// Imports
use {
	dynatos_context::OpaqueHandle,
	dynatos_util::{ObjectGet, ObjectSetProp},
	wasm_bindgen::prelude::wasm_bindgen,
};

/// Extension trait to add an context to an object
// TODO: Allow removing context handles?
#[extend::ext(name = ObjectAttachContext)]
pub impl js_sys::Object {
	/// Provides and attaches a context to this object
	fn attach_context<T: 'static>(&self, value: T) {
		// Get the context handles array, or create it, if it doesn't exist
		// TODO: Use an static anonymous symbol?
		let prop_name = "__dynatos_ctx_handles";
		let ctx_handles = match self.get::<js_sys::Array>(prop_name) {
			Ok(ctx_handles) => ctx_handles,
			Err(dynatos_util::GetError::WrongType(err)) => panic!("Contexts array was the wrong type: {err:?}"),
			Err(dynatos_util::GetError::Missing) => {
				let ctx_handles = js_sys::Array::new();
				self.set_prop(prop_name, &ctx_handles);
				ctx_handles
			},
		};

		// Then push the context handle
		let handle = dynatos_context::provide::<T>(value).into_opaque();
		let handle = WasmContextHandle(handle);
		ctx_handles.push(&handle.into());
	}
}

/// Extension trait to add an context to an object
// TODO: Allow removing context handles?
#[extend::ext(name = ObjectWithContext)]
pub impl<O> O
where
	O: AsRef<js_sys::Object>,
{
	/// Provides and attaches a context to this object
	fn with_context<T: 'static>(self, value: T) -> Self {
		self.as_ref().attach_context(value);
		self
	}
}

/// A wasm `OpaqueHandle` type.
#[wasm_bindgen]
#[expect(dead_code, reason = "We just want to keep the field alive, not use it")]
struct WasmContextHandle(OpaqueHandle);
