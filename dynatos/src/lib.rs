//! Dynatos framework

// TODO: Use a single object, `__dynatos` with all of the effects, instead of a
//       property for each?

// Imports
use {
	dynatos_reactive::Effect,
	dynatos_util::ObjectDefineProperty,
	std::cell::RefCell,
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
		F: Fn() -> N + 'static,
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
			if let Some(prev_child) = &*prev_child.borrow() {
				node.remove_child(prev_child.as_ref())
					.expect("Reactive child was removed");
			}

			// And set the child
			let child = f();
			node.append_child(child.as_ref())
				.expect("Unable to append reactive child");
			*prev_child.borrow_mut() = Some(child);
		}));

		// Then set it

		self.as_ref().define_property("__dynatos_child_effect", child_effect);
	}

	/// Adds a dynamic child to this node.
	///
	/// Returns the node, for chaining
	fn with_dyn_child<F, N>(self, f: F) -> Self
	where
		F: Fn() -> N + 'static,
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
