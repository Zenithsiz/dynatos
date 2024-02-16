//! Dynatos framework

// Features
#![feature(let_chains, unboxed_closures, associated_type_bounds)]

// Modules
pub mod dyn_element;

// Exports
pub use dyn_element::dyn_element;

// Imports
use {
	dynatos_html::html,
	dynatos_reactive::Effect,
	dynatos_util::{ObjectGet, ObjectRemoveProp, ObjectSetProp, TryOrReturnExt, WeakRef},
	std::cell::RefCell,
	wasm_bindgen::{prelude::wasm_bindgen, JsValue},
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

/// Extension trait to add a reactive child to an node
#[extend::ext(name = NodeDynChild)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
	/// Adds a dynamic child to this node
	fn dyn_child<C>(&self, child: C)
	where
		C: AsDynNode + 'static,
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
			let new_child = child.as_node();
			let new_child = new_child.get();

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
	fn with_dyn_child<C>(self, child: C) -> Self
	where
		C: AsDynNode + 'static,
	{
		self.dyn_child(child);
		self
	}
}

/// Type used for the output of [`AsDynNode`].
///
/// This allows [`AsDynNode`] to work with both owned
/// values, as well as `Option`s of those owned values.
pub trait AsOptNode {
	fn get(&self) -> Option<&web_sys::Node>;
}

impl<N> AsOptNode for &N
where
	N: AsOptNode,
{
	fn get(&self) -> Option<&web_sys::Node> {
		N::get(self)
	}
}

impl<N> AsOptNode for Option<N>
where
	N: AsOptNode,
{
	fn get(&self) -> Option<&web_sys::Node> {
		self.as_ref().and_then(N::get)
	}
}

// TODO: Impl for `impl AsRef<web_sys::Node>` if we can get rid of
//       the conflict with the function impl
#[duplicate::duplicate_item(
	Ty;
	[web_sys::Node];
	[web_sys::Element];
)]
impl AsOptNode for Ty {
	fn get(&self) -> Option<&web_sys::Node> {
		Some(self)
	}
}

/// Trait for values accepted by [`NodeDynChild`].
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `impl Fn() -> Option<N>`
/// - `N`
/// - `Option<N>`
/// Where `N` is a node type.
pub trait AsDynNode {
	/// The inner node type.
	type Node<'a>: AsOptNode
	where
		Self: 'a;

	/// Retrieves / Computes the inner node
	fn as_node(&self) -> Self::Node<'_>;
}

impl<F, N> AsDynNode for F
where
	F: Fn() -> N,
	N: AsOptNode,
{
	type Node<'a> = N where Self: 'a;

	fn as_node(&self) -> Self::Node<'_> {
		self()
	}
}

// TODO: Impl for `impl AsRef<web_sys::Node>` if we can get rid of
//       the conflict with the function impl
#[duplicate::duplicate_item(
	Ty;
	[web_sys::Node];
	[web_sys::Element];
)]
impl AsDynNode for Ty {
	type Node<'a> = &'a Ty;

	fn as_node(&self) -> Self::Node<'_> {
		self
	}
}

#[duplicate::duplicate_item(
	Ty;
	[web_sys::Node];
	[web_sys::Element];
)]
impl AsDynNode for Option<Ty> {
	type Node<'a> = Option<&'a Ty>;

	fn as_node(&self) -> Self::Node<'_> {
		self.as_ref()
	}
}

/// Extension trait to add reactive text to a node
#[extend::ext(name = NodeDynText)]
pub impl<T> T
where
	T: AsRef<web_sys::Node>,
{
	/// Adds dynamic text to this node
	fn dyn_text<U>(&self, text: U)
	where
		U: WithDynText + 'static,
	{
		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		let node = WeakRef::new(self.as_ref());
		let text_effect = Effect::try_new(move || {
			// Try to get the node
			let node = node.get().or_return()?;

			// And set the text content
			text.with_text(|text| node.set_text_content(text));
		})
		.or_return()?;

		// Then set it
		self.as_ref().attach_effect(text_effect);
	}

	/// Adds dynamic text to this node.
	///
	/// Returns the node, for chaining
	fn with_dyn_text<U>(self, text: U) -> Self
	where
		U: WithDynText + 'static,
	{
		self.dyn_text(text);
		self
	}
}

/// Trait for values accepted by [`NodeDynText`].
///
/// This allows it to work with the following types:
/// - `impl Fn() -> N`
/// - `impl Fn() -> Option<N>`
/// - `N`
/// - `Option<N>`
/// Where `N` is a text type.
pub trait WithDynText {
	/// Calls `f` with the inner text
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O;
}

impl<FT, T> WithDynText for FT
where
	FT: Fn() -> T,
	T: WithDynText,
{
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		let text = self();
		text.with_text(f)
	}
}

#[duplicate::duplicate_item(
	Ty;
	[str];
	[&'static str];
	[String];
)]
impl WithDynText for Ty {
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		f(Some(self))
	}
}

#[duplicate::duplicate_item(
	Ty;
	[&'static str];
	[String];
)]
impl WithDynText for Option<Ty> {
	fn with_text<F, O>(&self, f: F) -> O
	where
		F: FnOnce(Option<&str>) -> O,
	{
		match self {
			Some(s) => f(Some(s)),
			None => f(None),
		}
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

/// Extension trait to add reactive prop to an object
#[extend::ext(name = ObjectDynProp)]
pub impl<T> T
where
	T: AsRef<js_sys::Object>,
{
	/// Adds a dynamic property to this object
	fn dyn_prop<F, K, V>(&self, f: F)
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
		self.dyn_prop(f);
		self
	}

	/// Adds a dynamic property to this object, where only the value is dynamic.
	fn dyn_prop_value<F, K, V>(&self, key: K, f: F)
	where
		F: Fn() -> Option<V> + 'static,
		K: AsRef<str> + Copy + 'static,
		V: Into<JsValue>,
	{
		self.dyn_prop(move || (key, f()));
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
		self.dyn_prop_value(key, f);
		self
	}
}

/// A wasm `Effect` type.
#[wasm_bindgen]
struct WasmEffect(Effect);
