//! Object effect attaching

// Imports
use {
	dynatos_reactive::{Effect, EffectRun},
	dynatos_web::{ObjectGet, ObjectSetProp},
	dynatos_web::types::{Object, cfg_ssr_expr},
};

/// Extension trait to add an effect to an object
// TODO: Allow removing effects?
#[extend::ext(name = ObjectAttachEffect)]
pub impl Object {
	/// Attaches an effect to this object
	fn attach_effect<F>(&self, effect: Effect<F>)
	where
		F: ?Sized + EffectRun,
	{
		// TODO: Use an static anonymous symbol?
		let prop_name = "__dynatos_effects";

		cfg_ssr_expr!(
			ssr = {
				use {
					dynatos_inheritance::{FromFields, Value},
					dynatos_web_ssr::{ObjectAttachEffectEffects, ObjectAttachEffectEffectsFields},
					std::{collections::HashMap, sync::nonpoison::Mutex},
				};

				let effects = match self.get::<ObjectAttachEffectEffects>(prop_name) {
					Ok(effects) => effects,
					Err(dynatos_web::GetError::WrongType(err)) => panic!("Effects map was the wrong type: {err:?}"),
					Err(dynatos_web::GetError::Missing) => {
						let effects = ObjectAttachEffectEffects::from_fields((
							ObjectAttachEffectEffectsFields {
								effects: Mutex::new(HashMap::new()),
							},
							<Object as Value>::Fields::default(),
						));
						self.set_prop(prop_name, effects.clone());

						effects
					},
				};

				effects.fields().effects.lock().insert(effect.id(), effect.unsize());
			},
			csr = {
				/// A wasm `Effect` type.
				#[wasm_bindgen::prelude::wasm_bindgen]
				#[expect(dead_code, reason = "We just want to keep the field alive, not use it")]
				struct WasmEffect(Effect);

				// Get the effects map, or create it, if it doesn't exist
				let effects = match self.get::<js_sys::Map>(prop_name) {
					Ok(effects) => effects,
					Err(dynatos_web::GetError::WrongType(err)) => panic!("Effects map was the wrong type: {err:?}"),
					Err(dynatos_web::GetError::Missing) => {
						let effects = js_sys::Map::new();
						self.set_prop(prop_name, &effects);
						effects
					},
				};

				// Then push the effects
				let id = effect.id();
				let effect = WasmEffect(effect.unsize());
				effects.set(&id.into(), &effect.into());
			}
		)
	}
}

/// Extension trait to add an effect to an object
#[extend::ext(name = ObjectWithEffect)]
pub impl<O> O
where
	O: AsRef<Object>,
{
	/// Attaches an effect to this object.
	///
	/// Returns the object, for chaining
	fn with_effect<F>(self, effect: Effect<F>) -> Self
	where
		F: ?Sized + EffectRun,
	{
		self.as_ref().attach_effect(effect);
		self
	}
}
