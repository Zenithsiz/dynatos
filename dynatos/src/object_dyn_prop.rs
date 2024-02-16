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
pub impl<T> T
where
	T: AsRef<js_sys::Object>,
{
	/// Adds a dynamic property to this object
	fn add_dyn_prop<F, K, V>(&self, f: F)
	where
		F: Fn() -> (K, Option<V>) + 'static,
		K: AsRef<str>,
		V: Into<JsValue>,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the object.
		//       Otherwise, the object will be keeping us alive, while we keep
		//       the object alive, causing a leak.
		let object = WeakRef::new(self.as_ref());
		let prop_effect = Effect::try_new(move || {
			// Try to get the object
			let object = object.get().or_return()?;

			// And set the property
			let (key, value) = f();
			let key = key.as_ref();
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
		self.as_ref().attach_effect(prop_effect);
	}

	/// Adds a dynamic property to this object.
	///
	/// Returns the object, for chaining
	fn with_dyn_prop<F, K, V>(self, f: F) -> Self
	where
		F: Fn() -> (K, Option<V>) + 'static,
		K: AsRef<str>,
		V: Into<JsValue>,
	{
		self.add_dyn_prop(f);
		self
	}

	/// Adds a dynamic property to this object, where only the value is dynamic.
	fn add_dyn_prop_value<F, K, V>(&self, key: K, f: F)
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + Copy + 'static,
		V: Into<JsValue>,
	{
		self.add_dyn_prop(move || (key, f()));
	}

	/// Adds a dynamic property to this object, where only the value is dynamic.
	///
	/// Returns the object, for chaining
	fn with_dyn_prop_value<F, K, V>(self, key: K, f: F) -> Self
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + Copy + 'static,
		V: Into<JsValue>,
	{
		self.add_dyn_prop_value(key, f);
		self
	}
}
