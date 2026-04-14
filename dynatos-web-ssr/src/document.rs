//! Document

// Imports
use {
	crate::{
		Comment,
		Element,
		EventTarget,
		HtmlBodyElement,
		HtmlElement,
		HtmlHeadElement,
		Location,
		Node,
		Object,
		Text,
		WebError,
		event_target::EventTargetFields,
		node::NodeFields,
		object::ObjectFields,
	},
	dynatos_inheritance::{FromFields, Value},
	std::sync::nonpoison::Mutex,
};

dynatos_inheritance::value! {
	pub struct Document(Node, EventTarget, Object): Send + Sync + Debug {
		title: Mutex<String>,
		head: HtmlHeadElement,
		body: HtmlBodyElement,
		location: Location,
	}
	impl Self {}
}

impl Document {
	#[must_use]
	pub fn new(location: String) -> Self {
		Self::from_fields((
			DocumentFields {
				title:    Mutex::new(String::new()),
				head:     HtmlHeadElement::new(),
				body:     HtmlBodyElement::new(),
				location: Location::new(location),
			},
			NodeFields::new("#document"),
			EventTargetFields::default(),
			ObjectFields::default(),
		))
	}

	pub fn head(&self) -> Result<HtmlHeadElement, WebError> {
		Ok(self.head_ref().clone())
	}

	pub fn body(&self) -> Result<HtmlBodyElement, WebError> {
		Ok(self.body_ref().clone())
	}

	#[must_use]
	#[expect(clippy::missing_const_for_fn, reason = "False positive")]
	pub fn head_ref(&self) -> &HtmlHeadElement {
		&self.fields().head
	}

	#[must_use]
	#[expect(clippy::missing_const_for_fn, reason = "False positive")]
	pub fn body_ref(&self) -> &HtmlBodyElement {
		&self.fields().body
	}

	#[must_use]
	pub fn title(&self) -> String {
		self.fields().title.lock().clone()
	}

	pub fn set_title(&self, title: &str) {
		title.clone_into(&mut self.fields().title.lock());
	}

	#[must_use]
	pub fn create_text_node(&self, contents: &str) -> Text {
		Text::new(Some(contents.to_owned()))
	}

	#[must_use]
	pub fn create_comment(&self, contents: &str) -> Comment {
		Comment::new(Some(contents.to_owned()))
	}

	pub fn create_element_ns(&self, _namespace: Option<&str>, name: &str) -> Result<Element, WebError> {
		// TODO: Check the namespace?
		Ok(HtmlElement::new(name).into())
	}

	pub fn location(&self) -> Result<Location, WebError> {
		Ok(self.location_ref().clone())
	}

	#[must_use]
	#[expect(clippy::missing_const_for_fn, reason = "False positive")]
	pub fn location_ref(&self) -> &Location {
		&self.fields().location
	}
}
