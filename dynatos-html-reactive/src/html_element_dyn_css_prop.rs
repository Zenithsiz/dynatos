//! Html element reactive css property

// Imports
use {
	crate::{ObjectAttachEffect, WithDynPred, WithDynText},
	dynatos_html::WeakRef,
	dynatos_reactive::Effect,
	dynatos_util::TryOrReturnExt,
};

/// Extension trait to add reactive css properties to an html element
#[extend::ext(name = HtmlElementDynCssProp)]
pub impl web_sys::HtmlElement {
	/// Adds a dynamic css property to this element
	#[track_caller]
	fn set_dyn_css_prop<K, V>(&self, key: K, value: V)
	where
		K: AsRef<str> + 'static,
		V: WithDynText + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the element.
		//       Otherwise, the element will be keeping us alive, while we keep
		//       the element alive, causing a leak.
		let element = WeakRef::new(self);
		let prop_effect = Effect::try_new(move || {
			// Try to get the element
			let element = element.get().or_return()?;

			// And set the property
			let key = key.as_ref();
			value.with_text(|value| match value {
				Some(value) => element
					.style()
					.set_property(key, value)
					.unwrap_or_else(|err| panic!("Unable to set css property {key:?} with value {value:?}: {err:?}")),
				None =>
					_ = element
						.style()
						.remove_property(key)
						.unwrap_or_else(|err| panic!("Unable to remove css property {key:?}: {err:?}")),
			});
		})
		.or_return()?;

		// Then set it
		self.attach_effect(prop_effect);
	}

	/// Adds a dynamic css property to this element, with an empty value, given a predicate
	#[track_caller]
	fn set_dyn_css_prop_if<K, P>(&self, key: K, pred: P)
	where
		K: AsRef<str> + 'static,
		P: WithDynPred + 'static,
	{
		self.set_dyn_css_prop(key, move || pred.eval().then_some(""));
	}
}

/// Extension trait to add reactive css property to an element
#[extend::ext(name = HtmlElementWithDynCssProp)]
pub impl<E> E
where
	E: AsRef<web_sys::HtmlElement>,
{
	/// Adds a dynamic css property to this element, where only the value is dynamic.
	///
	/// Returns the element, for chaining
	#[track_caller]
	fn with_dyn_css_prop<K, V>(self, key: K, value: V) -> Self
	where
		K: AsRef<str> + 'static,
		V: WithDynText + 'static,
	{
		self.as_ref().set_dyn_css_prop(key, value);
		self
	}

	/// Adds a dynamic css property to this element, without a value, given a predicate
	///
	/// Returns the element, for chaining
	#[track_caller]
	fn with_dyn_css_prop_if<K, P>(self, key: K, pred: P) -> Self
	where
		K: AsRef<str> + 'static,
		P: WithDynPred + 'static,
	{
		self.as_ref().set_dyn_css_prop_if(key, pred);
		self
	}
}
