//! Object effect attaching

// Imports
use {
	dynatos_reactive::Effect,
	dynatos_util::{ObjectGet, ObjectSetProp},
	wasm_bindgen::prelude::wasm_bindgen,
};

/// Extension trait to add an effect to an object
// TODO: Allow removing effects?
#[extend::ext(name = ObjectAttachEffect)]
pub impl<T> T
where
	T: AsRef<js_sys::Object>,
{
	/// Attaches an effect to this object
	fn attach_effect(&self, effect: Effect) {
		// Get the effects array, or create it, if it doesn't exist
		// TODO: Use an static anonymous symbol?
		let prop_name: &str = "__dynatos_effects";
		let obj = self.as_ref();
		let effects = match obj.get::<js_sys::Map>(prop_name) {
			Ok(effects) => effects,
			Err(dynatos_util::GetError::WrongType(err)) => panic!("Effects array was the wrong type: {err:?}"),
			Err(dynatos_util::GetError::Missing) => {
				let effects = js_sys::Map::new();
				obj.set_prop(prop_name, &effects);
				effects
			},
		};

		// Then push the effects
		let effect_key = effect.inner_ptr();
		let effect = WasmEffect(effect);
		effects.set(&effect_key.into(), &effect.into());
	}

	/// Attaches an effect to this object.
	///
	/// Returns the object, for chaining
	fn with_effect(self, effect: Effect) -> Self {
		self.attach_effect(effect);
		self
	}
}

/// A wasm `Effect` type.
#[wasm_bindgen]
struct WasmEffect(Effect);
