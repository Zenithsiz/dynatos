//! Dynatos framework

// Features
#![feature(let_chains)]

// Modules
pub mod dyn_element;

// Exports
pub use dyn_element::dyn_element;

// Imports
use {
	dynatos_html::html,
	dynatos_reactive::Effect,
	dynatos_util::{ObjectGet, ObjectSet, TryOrReturnExt, WeakRef},
	std::cell::RefCell,
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
				obj.set(prop_name, &effects);
				effects
			},
		};

		// Then push the effects
		let effect_key = effect.inner_ptr();
		let effect = WasmEffect(effect);
		effects.set(&effect_key.into(), &effect.into());
	}

	/// Attaches an effect to this node.
	///
	/// Returns the node, for chaining
	fn with_effect(self, effect: Effect) -> Self {
		self.attach_effect(effect);
		self
	}
}

/// Extension trait to add a reactive child to an node
#[extend::ext(name = NodeDynChild)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
	/// Adds a dynamic child to this node
	fn dyn_child<F, N>(&self, f: F)
	where
		F: Fn() -> Option<N> + 'static,
		N: AsRef<web_sys::Node>,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		// Note: We have an empty `<template>` so that we can track the position
		//       of the node, in case of `f` returning `None`.
		// TODO: Find a better solution for when `f` returns `None` that doesn't involve
		//       adding an element to the dom?
		let node = WeakRef::new(self.as_ref());
		let prev_child = RefCell::new(None::<web_sys::Node>);
		let empty_child = web_sys::Node::from(html::template());
		let child_effect = Effect::try_new(move || {
			// Try to get the node
			let node = node.get().or_return()?;

			// Get the new child
			// Note: If the new child already exists,
			let new_child = f();
			let new_child = new_child.as_ref().map(N::as_ref);

			// Check if someone's messed with our previous child
			// TODO: At this point should we give up, since we lost the position?
			//       The behavior of trying again might be worse.
			let mut prev_child = prev_child.borrow_mut();
			if let Some(child) = &*prev_child &&
				!node.contains(Some(child))
			{
				tracing::warn!("Reactive child was removed externally, re-inserting");
				*prev_child = None;
			}

			// Then check if we need to substitute in the empty child
			let new_child = match new_child {
				// If the new child is the same as the old one, we can return
				Some(child) if prev_child.as_ref() == Some(child) => return,

				// Otherwise, if this is a duplicate node, warn and use an empty child
				// Note: The typical browser behavior would be to remove the previous
				//       child, then add ours. Unfortunately, removing other nodes might
				//       cause another dyn child to panic due to it's previous node being
				//       missing.
				Some(child) if node.contains(Some(child)) => {
					tracing::warn!("Attempted to add a reactive node multiple times");
					&empty_child
				},

				// Otherwise, use the new child
				Some(child) => child,

				// Finally, if no child was given, use the empty child
				None => &empty_child,
			};

			// Then update the node
			match &mut *prev_child {
				// If we already have a node, replace it
				Some(prev_child) => node
					.replace_child(new_child, prev_child)
					.expect("Unable to replace reactive child"),

				// Otherwise, we're running for the first time, so append the child
				None => node.append_child(new_child).expect("Unable to append reactive child"),
			};

			*prev_child = Some(new_child.clone());
		})
		.or_return()?;

		// Then set it
		self.as_ref().attach_effect(child_effect);
	}

	/// Adds a dynamic child to this node.
	///
	/// Returns the node, for chaining
	fn with_dyn_child<F, N>(self, f: F) -> Self
	where
		F: Fn() -> Option<N> + 'static,
		N: AsRef<web_sys::Node> + 'static,
	{
		self.dyn_child(f);
		self
	}
}

/// Extension trait to add reactive text to a node
#[extend::ext(name = NodeDynText)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
	/// Adds dynamic text to this node
	fn dyn_text<F, S>(&self, f: F)
	where
		F: Fn() -> Option<S> + 'static,
		S: AsRef<str>,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		let node = WeakRef::new(self.as_ref());
		let text_content_effect = Effect::try_new(move || {
			// Try to get the node
			let node = node.get().or_return()?;

			// And set the text content
			match f() {
				Some(s) => node.set_text_content(Some(s.as_ref())),
				None => node.set_text_content(None),
			}
		})
		.or_return()?;

		// Then set it
		self.as_ref().attach_effect(text_content_effect);
	}

	/// Adds dynamic text to this node.
	///
	/// Returns the node, for chaining
	fn with_dyn_text<F, S>(self, f: F) -> Self
	where
		F: Fn() -> Option<S> + 'static,
		S: AsRef<str>,
	{
		self.dyn_text(f);
		self
	}
}

/// Extension trait to add reactive attribute to an element
#[extend::ext(name = ElementDynAttr)]
pub impl<T> T
where
	T: AsRef<web_sys::Element>,
{
	/// Adds a dynamic attribute to this element
	fn dyn_attr<F, K, V>(&self, f: F)
	where
		F: Fn() -> (K, Option<V>) + 'static,
		K: AsRef<str>,
		V: AsRef<str>,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the element.
		//       Otherwise, the element will be keeping us alive, while we keep
		//       the element alive, causing a leak.
		let element = WeakRef::new(self.as_ref());
		let attr_effect = Effect::try_new(move || {
			// Try to get the element
			let element = element.get().or_return()?;

			// And set the attribute
			let (key, value) = f();
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
		self.as_ref().attach_effect(attr_effect);
	}

	/// Adds a dynamic attribute to this element.
	///
	/// Returns the element, for chaining
	fn with_dyn_attr<F, K, V>(self, f: F) -> Self
	where
		F: Fn() -> (K, Option<V>) + 'static,
		K: AsRef<str>,
		V: AsRef<str>,
	{
		self.dyn_attr(f);
		self
	}

	/// Adds a dynamic attribute to this element, where only the value is dynamic.
	fn dyn_attr_value<F, K, V>(&self, key: K, f: F)
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + Copy + 'static,
		V: AsRef<str>,
	{
		self.dyn_attr(move || (key, f()));
	}

	/// Adds a dynamic attribute to this element, where only the value is dynamic.
	///
	/// Returns the element, for chaining
	fn with_dyn_attr_value<F, K, V>(self, key: K, f: F) -> Self
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + Copy + 'static,
		V: AsRef<str>,
	{
		self.dyn_attr_value(key, f);
		self
	}

	/// Adds a dynamic attribute to this element, without a value, given a predicate
	fn dyn_attr_if<F, K>(&self, key: K, f: F)
	where
		F: Fn() -> bool + 'static,
		K: AsRef<str> + Copy + 'static,
	{
		self.dyn_attr(move || (key, f().then_some("")));
	}

	/// Adds a dynamic attribute to this element, without a value, given a predicate
	///
	/// Returns the element, for chaining
	fn with_dyn_attr_if<F, K>(self, key: K, f: F) -> Self
	where
		F: Fn() -> bool + 'static,
		K: AsRef<str> + Copy + 'static,
	{
		self.dyn_attr_if(key, f);
		self
	}
}


/// A wasm `Effect` type.
#[wasm_bindgen]
struct WasmEffect(Effect);
