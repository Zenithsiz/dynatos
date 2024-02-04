//! Dynatos framework

// TODO: Use a single object, `__dynatos` with all of the effects, instead of a
//       property for each?

// Imports
use {
	dynatos_reactive::Effect,
	dynatos_util::ObjectDefineProperty,
	std::cell::{Cell, RefCell},
	wasm_bindgen::{prelude::wasm_bindgen, JsCast},
};

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
		N: AsRef<web_sys::Node> + 'static,
	{
		/// Effect to attach to the node
		#[wasm_bindgen]
		struct ChildEffect(Effect);

		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		let node = WeakRef::new(self.as_ref());
		let prev_child = RefCell::new(None::<N>);
		let child_effect = ChildEffect(Effect::new(move || {
			// Try to get the node
			let Some(node) = node.deref() else {
				return;
			};
			let node = node.dyn_into::<web_sys::Node>().expect("Should be Node");

			// Remove the previous child, if it exists
			if let Some(prev_child) = prev_child.borrow_mut().take() {
				node.remove_child(prev_child.as_ref())
					.expect("Reactive child was removed");
			}

			// And set the child, if any
			if let Some(child) = f() {
				node.append_child(child.as_ref())
					.expect("Unable to append reactive child");
				*prev_child.borrow_mut() = Some(child)
			}
		}));

		// Get a unique id for the property name
		// Note: Since a node may have multiple reactive children,
		//       we can't use a single property name for this
		// TODO: Technically, two different threads may set this and
		//       get the same id, check alternatives.
		thread_local! {
			static PROP_IDX: Cell<usize> = Cell::new(0);
		}
		let prop_idx = PROP_IDX.get();
		PROP_IDX.set(prop_idx + 1);

		// Then set it
		let prop = format!("__dynatos_child_effect_{}", prop_idx);
		self.as_ref().define_property(&prop, child_effect);
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
		/// Effect to attach to the node
		#[wasm_bindgen]
		struct TextContentEffect(Effect);

		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the node.
		//       Otherwise, the node will be keeping us alive, while we keep
		//       the node alive, causing a leak.
		let node = WeakRef::new(self.as_ref());
		let text_content_effect = TextContentEffect(Effect::new(move || {
			// Try to get the node
			let Some(node) = node.deref() else {
				return;
			};
			let node = node.dyn_into::<web_sys::Node>().expect("Should be node");

			// And set the text content
			match f() {
				Some(s) => node.set_text_content(Some(s.as_ref())),
				None => node.set_text_content(None),
			}
		}));

		// Then set it
		self.as_ref()
			.define_property("__dynatos_text_content_effect", text_content_effect);
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
		/// Effect to attach to the element
		#[wasm_bindgen]
		struct AttrEffect(Effect);

		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the element.
		//       Otherwise, the element will be keeping us alive, while we keep
		//       the element alive, causing a leak.
		let element = WeakRef::new(self.as_ref());
		let attr_effect = AttrEffect(Effect::new(move || {
			// Try to get the element
			let Some(element) = element.deref() else {
				return;
			};
			let element = element.dyn_into::<web_sys::Element>().expect("Should be element");

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
		}));

		// Then set it
		self.as_ref().define_property("__dynatos_attr_effect", attr_effect);
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
}

#[wasm_bindgen]
extern "C" {
	/// Weak reference.
	///
	/// [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WeakRef)
	#[wasm_bindgen(js_name = WeakRef)]
	pub type WeakRef;

	/// Constructor
	///
	/// [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WeakRef/WeakRef)
	#[wasm_bindgen(constructor)]
	pub fn new(target: &js_sys::Object) -> WeakRef;

	/// Dereference method
	///
	/// [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WeakRef/deref)
	#[wasm_bindgen(method, js_name = deref)]
	pub fn deref(this: &WeakRef) -> Option<js_sys::Object>;
}
