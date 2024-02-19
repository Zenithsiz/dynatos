//! Object reactive property

// Imports
use {
	crate::ObjectAttachEffect,
	dynatos_reactive::Effect,
	dynatos_util::{ObjectRemoveProp, ObjectSetProp, TryOrReturnExt, WeakRef},
	std::cell::RefCell,
	wasm_bindgen::JsValue,
};

/// Extension trait to add reactive prop to an object
#[extend::ext(name = ObjectDynProp)]
pub impl js_sys::Object {
	/// Adds a dynamic property to this object
	fn add_dyn_prop<F, K, V>(&self, f: F)
	where
		F: Fn() -> (K, Option<V>) + 'static,
		K: AsRef<str>,
		V: Into<JsValue>,
	{
		// The object we're attaching to
		// Note: It's important that we only keep a `WeakRef` to the object.
		//       Otherwise, the object will be keeping us alive, while we keep
		//       the object alive, causing a leak.
		let object = WeakRef::new(self);

		// The last set key value.
		// Note: We don't actually store the previous value, since we don't
		//       have a need for it. Instead we just store if there was a value.
		let last_key_value = RefCell::new(None::<(String, bool)>);

		// Create the effect
		let prop_effect = Effect::try_new(move || {
			// Try to get the object
			let object = object.get().or_return()?;

			// Then get the property
			let (key, value) = f();
			let key = key.as_ref();

			// If we've already set a property with a value, and the current
			// key is different, remove the old one first.
			if let Some((ref last_key, has_value)) = *last_key_value.borrow() &&
				has_value && last_key != key
			{
				object.remove_prop(last_key);
			}

			// Then update the last key-value
			*last_key_value.borrow_mut() = Some((key.to_owned(), value.is_some()));

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

	/// Adds a dynamic property to this object, where only the value is dynamic.
	fn add_dyn_prop_value<F, K, V>(&self, key: K, f: F)
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + Copy + 'static,
		V: Into<JsValue>,
	{
		self.add_dyn_prop(move || (key, f()));
	}
}

/// Extension trait to add reactive prop to an object
#[extend::ext(name = ObjectWithDynProp)]
pub impl<O> O
where
	O: AsRef<js_sys::Object>,
{
	/// Adds a dynamic property to this object.
	///
	/// Returns the object, for chaining
	fn with_dyn_prop<F, K, V>(self, f: F) -> Self
	where
		F: Fn() -> (K, Option<V>) + 'static,
		K: AsRef<str>,
		V: Into<JsValue>,
	{
		self.as_ref().add_dyn_prop(f);
		self
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
		self.as_ref().add_dyn_prop_value(key, f);
		self
	}
}
