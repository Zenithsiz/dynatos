//! Utilities for [`dynatos`]

// Features
#![feature(decl_macro)]

// Modules
mod event_listener;

// Exports
pub use event_listener::{ev, ElementEventListener, EventListener};

// Imports
use {
	std::fmt,
	wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue},
};

/// Extension trait to be able to use `.context` on `Result<T, JsValue>`.
#[extend::ext(name = JsResultContext)]
pub impl<T> Result<T, JsValue> {
	fn context<C>(self, context: C) -> Result<T, anyhow::Error>
	where
		C: fmt::Display + Send + Sync + 'static,
	{
		self.map_err(|err| {
			let err = format!("{err:?}");
			let err = anyhow::Error::msg(err);
			err.context(context)
		})
	}
}

/// Extension trait to define a property on an object
#[extend::ext(name = ObjectDefineProperty)]
pub impl js_sys::Object {
	fn define_property<T>(&self, property: &str, value: T)
	where
		T: Into<JsValue>,
	{
		/// Object descriptor to use for `Object::define_property`.
		#[wasm_bindgen]
		pub struct ObjectDescriptor {
			value: JsValue,
		}

		#[wasm_bindgen]
		impl ObjectDescriptor {
			#[wasm_bindgen(getter)]
			pub fn value(&self) -> JsValue {
				self.value.clone()
			}
		}


		// Setup the descriptor
		let descriptor = ObjectDescriptor { value: value.into() };
		let descriptor = JsValue::from(descriptor);
		let descriptor = descriptor.dyn_into::<js_sys::Object>().expect("Should be an object");

		// Then define it.
		js_sys::Object::define_property(self, &JsValue::from_str(property), &descriptor);
	}
}

/// Extension trait to set the text content in a builder-style.
#[extend::ext_sized(name = ElementWithTextContent)]
pub impl web_sys::Element {
	fn with_text_content<T>(self, text: T) -> Self
	where
		T: AsTextContent,
	{
		self.set_text_content(text.as_text_content());
		self
	}
}

/// Types that may be used for [`ElementWithTextContent`]
pub trait AsTextContent {
	/// Returns the text content
	fn as_text_content(&self) -> Option<&str>;
}

// Note: We only add a single impl for `Option<_>` to ensure
//       that calling `with_text_content(None)` works without
//       specifying any type annotations
#[duplicate::duplicate_item(
	Ty body;
	[ &'_ str ] [ Some(self) ];
	[ String ] [ Some(self.as_str()) ];
	[ Option<String> ] [ self.as_deref() ];
)]
impl AsTextContent for Ty {
	fn as_text_content(&self) -> Option<&str> {
		body
	}
}

/// Extension trait to add children to an element
#[extend::ext_sized(name = ElementWithChildren)]
pub impl web_sys::Element {
	fn with_children<C>(self, children: C) -> Result<Self, JsValue>
	where
		C: Children,
	{
		children.append_all(&self).map(|()| self)
	}
}

/// Types that may be used for [`ElementWithTextContent`]
pub trait Children {
	/// Appends all children in this type
	fn append_all(self, element: &web_sys::Element) -> Result<(), JsValue>;
}

impl<const N: usize> Children for [web_sys::Element; N] {
	fn append_all(self, element: &web_sys::Element) -> Result<(), JsValue> {
		for child in self {
			element.append_child(&child)?;
		}

		Ok(())
	}
}

/// Extension trait to add an attribute in a builder-style.
#[extend::ext_sized(name = ElementWithAttr)]
pub impl web_sys::Element {
	fn with_attr<A, V>(self, attr: A, value: V) -> Result<Self, JsValue>
	where
		A: AsRef<str>,
		V: AsRef<str>,
	{
		self.set_attribute(attr.as_ref(), value.as_ref()).map(|()| self)
	}
}
