//! Element reactive attribute

// Imports
use {
	crate::{ObjectAttachEffect, WithDynPred, WithDynText},
	dynatos_reactive::Effect,
	dynatos_sync_types::SyncBounds,
	dynatos_util::TryOrReturnExt,
	js_sys::WeakRef,
};

/// Extension trait to add reactive attribute to an element
#[extend::ext(name = ElementDynAttr)]
pub impl web_sys::Element {
	/// Adds a dynamic attribute to this element
	#[track_caller]
	fn set_dyn_attr<K, V>(&self, key: K, value: V)
	where
		K: SyncBounds + AsRef<str> + 'static,
		V: SyncBounds + WithDynText + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the element.
		//       Otherwise, the element will be keeping us alive, while we keep
		//       the element alive, causing a leak.
		let element = WeakRef::new(self);
		let attr_effect = Effect::try_new(move || {
			// Try to get the element
			let element = element.deref().or_return()?;

			// And set the attribute
			let key = key.as_ref();
			value.with_text(|value| match value {
				Some(value) => element
					.set_attribute(key, value)
					.unwrap_or_else(|err| panic!("Unable to set attribute {key:?} with value {value:?}: {err:?}")),
				None => element
					.remove_attribute(key)
					.unwrap_or_else(|err| panic!("Unable to remove attribute {key:?}: {err:?}")),
			});
		})
		.or_return()?;

		// Then set it
		self.attach_effect(attr_effect);
	}

	/// Adds a dynamic attribute to this element, with an empty value, given a predicate
	#[track_caller]
	fn set_dyn_attr_if<K, P>(&self, key: K, pred: P)
	where
		K: SyncBounds + AsRef<str> + 'static,
		P: SyncBounds + WithDynPred + 'static,
	{
		self.set_dyn_attr(key, move || pred.eval().then_some(""));
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
	#[track_caller]
	fn with_dyn_attr<K, V>(self, key: K, value: V) -> Self
	where
		K: SyncBounds + AsRef<str> + 'static,
		V: SyncBounds + WithDynText + 'static,
	{
		self.as_ref().set_dyn_attr(key, value);
		self
	}

	/// Adds a dynamic attribute to this element, without a value, given a predicate
	///
	/// Returns the element, for chaining
	#[track_caller]
	fn with_dyn_attr_if<K, P>(self, key: K, pred: P) -> Self
	where
		K: SyncBounds + AsRef<str> + 'static,
		P: SyncBounds + WithDynPred + 'static,
	{
		self.as_ref().set_dyn_attr_if(key, pred);
		self
	}
}
