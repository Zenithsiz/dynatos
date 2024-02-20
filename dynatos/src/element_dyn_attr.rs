//! Element reactive attribute

// Imports
use {
	crate::ObjectAttachEffect,
	dynatos_reactive::Effect,
	dynatos_util::{TryOrReturnExt, WeakRef},
};

/// Extension trait to add reactive attribute to an element
#[extend::ext(name = ElementDynAttr)]
pub impl web_sys::Element {
	/// Adds a dynamic attribute to this element
	fn set_dyn_attr<F, K, V>(&self, key: K, f: F)
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + 'static,
		V: AsRef<str>,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the element.
		//       Otherwise, the element will be keeping us alive, while we keep
		//       the element alive, causing a leak.
		let element = WeakRef::new(self);
		let attr_effect = Effect::try_new(move || {
			// Try to get the element
			let element = element.get().or_return()?;

			// And set the attribute
			let value = f();
			let key = key.as_ref();
			match value {
				Some(value) => {
					let value = value.as_ref();
					element
						.set_attribute(key, value)
						.unwrap_or_else(|err| panic!("Unable to set attribute {key:?} with value {value:?}: {err:?}"))
				},
				None => element
					.remove_attribute(key)
					.unwrap_or_else(|err| panic!("Unable to remove attribute {key:?}: {err:?}")),
			}
		})
		.or_return()?;

		// Then set it
		self.attach_effect(attr_effect);
	}

	/// Adds a dynamic attribute to this element, with an empty value, given a predicate
	fn set_dyn_attr_if<F, K>(&self, key: K, f: F)
	where
		F: Fn() -> bool + 'static,
		K: AsRef<str> + 'static,
	{
		self.set_dyn_attr(key, move || f().then_some(""));
	}
}

/// Extension trait to add reactive attribute to an element
#[extend::ext(name = ElementWithDynAttr)]
pub impl<E> E
where
	E: AsRef<web_sys::Element>,
{
	/// Adds a dynamic attribute to this element, where only the value is dynamic.
	///
	/// Returns the element, for chaining
	fn with_dyn_attr<F, K, V>(self, key: K, f: F) -> Self
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + 'static,
		V: AsRef<str>,
	{
		self.as_ref().set_dyn_attr(key, f);
		self
	}

	/// Adds a dynamic attribute to this element, without a value, given a predicate
	///
	/// Returns the element, for chaining
	fn with_dyn_attr_if<F, K>(self, key: K, f: F) -> Self
	where
		F: Fn() -> bool + 'static,
		K: AsRef<str> + 'static,
	{
		self.as_ref().set_dyn_attr_if(key, f);
		self
	}
}
