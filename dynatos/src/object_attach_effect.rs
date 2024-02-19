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
pub impl js_sys::Object {
	/// Attaches an effect to this object
	fn attach_effect(&self, effect: Effect) {
		// Get the effects map, or create it, if it doesn't exist
		// TODO: Use an static anonymous symbol?
		let prop_name: &str = "__dynatos_effects";
		let effects = match self.get::<js_sys::Map>(prop_name) {
			Ok(effects) => effects,
			Err(dynatos_util::GetError::WrongType(err)) => panic!("Effects map was the wrong type: {err:?}"),
			Err(dynatos_util::GetError::Missing) => {
				let effects = js_sys::Map::new();
				self.set_prop(prop_name, &effects);
				effects
			},
		};

		// Then push the effects
		let effect_key = effect.inner_ptr();
		let effect = WasmEffect(effect);
		effects.set(&effect_key.into(), &effect.into());
	}
}

/// Extension trait to add an effect to an object
#[extend::ext(name = ObjectWithEffect)]
pub impl<O> O
where
	O: AsRef<js_sys::Object>,
{
	/// Attaches an effect to this object.
	///
	/// Returns the object, for chaining
	fn with_effect(self, effect: Effect) -> Self {
		self.as_ref().attach_effect(effect);
		self
	}
}

/// A wasm `Effect` type.
#[wasm_bindgen]
#[expect(dead_code, reason = "We just want to keep the field alive, not use it")]
struct WasmEffect(Effect);
