//! Object reactive property

// Imports
use {
	crate::ObjectAttachEffect,
	dynatos_reactive::Effect,
	dynatos_util::{ObjectRemoveProp, ObjectSetProp, TryOrReturnExt, WeakRef},
	wasm_bindgen::JsValue,
};

/// Extension trait to add reactive prop to an object
#[extend::ext(name = ObjectDynProp)]
pub impl js_sys::Object {
	/// Adds a dynamic property to this object, where only the value is dynamic.
	fn add_dyn_prop_value<F, K, V>(&self, key: K, f: F)
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + 'static,
		V: Into<JsValue>,
	{
		// The object we're attaching to
		// Note: It's important that we only keep a `WeakRef` to the object.
		//       Otherwise, the object will be keeping us alive, while we keep
		//       the object alive, causing a leak.
		let object = WeakRef::new(self);

		// Create the effect
		let prop_effect = Effect::try_new(move || {
			// Try to get the object
			let object = object.get().or_return()?;

			// Then get the property
			let value = f();
			let key = key.as_ref();

			// And finally set/remove the property
			match value {
				Some(value) => {
					object.set_prop(key, value);
				},
				None => {
					object.remove_prop(key);
				},
			}
		})
		.or_return()?;

		// Then set it
		self.attach_effect(prop_effect);
	}
}

/// Extension trait to add reactive prop to an object
#[extend::ext(name = ObjectWithDynProp)]
pub impl<O> O
where
	O: AsRef<js_sys::Object>,
{
	/// Adds a dynamic property to this object, where only the value is dynamic.
	///
	/// Returns the object, for chaining
	fn with_dyn_prop_value<F, K, V>(self, key: K, f: F) -> Self
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + 'static,
		V: Into<JsValue>,
	{
		self.as_ref().add_dyn_prop_value(key, f);
		self
	}
}
