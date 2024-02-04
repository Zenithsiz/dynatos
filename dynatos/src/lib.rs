//! Dynatos framework

// Imports
use {
	dynatos_reactive::Effect,
	dynatos_util::ObjectDefineProperty,
	wasm_bindgen::{prelude::wasm_bindgen, JsCast},
};

/// Extension trait to add a reactive element text to an element
#[extend::ext(name = ElementDynText)]
pub impl web_sys::Element {
	/// Adds dynamic text to this element
	fn dyn_text<F, S>(&self, f: F)
	where
		F: Fn() -> Option<S> + 'static,
		S: AsRef<str>,
	{
		/// Effect to attach to the element
		#[wasm_bindgen]
		struct TextContentEffect(Effect);

		// Create the value to attach
		// Note: It's important that we only keep a `WeakRef` to the element.
		//       Otherwise, the element will be keeping us alive, while we keep
		//       the element alive, causing a leak.
		let element = WeakRef::new(self);
		let text_content_effect = TextContentEffect(Effect::new(move || {
			// Try to get the element
			let Some(element) = element.deref() else {
				return;
			};
			let element = element.dyn_into::<web_sys::Element>().expect("Should be Element");

			// And set the text content
			match f() {
				Some(s) => element.set_text_content(Some(s.as_ref())),
				None => element.set_text_content(None),
			}
		}));

		// Then set it
		// TODO: Use a single object, `__dynatos` with all of the effects, instead of a
		//       property for each?
		self.define_property("__dynatos_text_content_effect", text_content_effect);
	}

	/// Adds dynamic text to this element.
	///
	/// Returns the element, for chaining
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
