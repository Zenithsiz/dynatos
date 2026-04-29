//! Html elements

// Imports
use {
	crate::{
		CssStyleProperties,
		Element,
		EventTarget,
		Node,
		Object,
		element::ElementFields,
		event_target::EventTargetFields,
		node::NodeFields,
		object::ObjectFields,
	},
	zutil_inheritance::{FromFields, Value},
};

zutil_inheritance::value! {
	pub struct HtmlElement(Element, Node, EventTarget, Object): Send + Sync + Debug + DefaultFields {
	}
	impl Self {}
}

impl HtmlElement {
	#[must_use]
	pub fn new(tag_name: impl Into<String>) -> Self {
		let tag_name = tag_name.into();
		let node_name = tag_name.to_uppercase();

		Self::from_fields((
			HtmlElementFields {},
			ElementFields::new(tag_name),
			NodeFields::new(node_name),
			EventTargetFields::default(),
			ObjectFields::default(),
		))
	}

	#[must_use]
	pub fn style(&self) -> CssStyleProperties {
		CssStyleProperties::new(self.clone())
	}
}

decl_html_elements! {
	new;

	HtmlBodyElement = "body",
	HtmlCanvasElement = "canvas",
	HtmlDialogElement = "dialog",
	HtmlHeadElement = "head",
	HtmlImageElement = "image",
	HtmlInputElement = "input",
	HtmlTextAreaElement = "textarea",
}

macro decl_html_elements($new:ident; $($Name:ident = $tag:literal),* $(,)?) {
	$(
		zutil_inheritance::value! {
			pub struct $Name(HtmlElement, Element, Node, EventTarget, Object): Send + Sync + Debug + DefaultFields {}
			impl Self {}
		}

		impl $Name {
			#[must_use]
			pub fn $new() -> Self {
				let tag_name = $tag;
				let node_name = tag_name.to_uppercase();

				Self::from_fields((
					<$Name as Value>::Fields::default(),
					HtmlElementFields::default(),
					ElementFields::new(tag_name),
					NodeFields::new(node_name),
					EventTargetFields::default(),
					ObjectFields::default(),
				))
			}
		}

		impl Default for $Name {
			fn default() -> Self {
				Self::$new()
			}
		}
	)*
}
