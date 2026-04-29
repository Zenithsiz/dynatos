//! Window

// Imports
use {
	crate::{Document, EventTarget, Object, WebError, event_target::EventTargetFields, object::ObjectFields},
	zutil_inheritance::{FromFields, Value},
};

zutil_inheritance::value! {
	pub struct Window(EventTarget, Object): Send + Sync + Debug {
		document: Document,
	}
	impl Self {}
}

impl Window {
	#[must_use]
	pub fn new(location: String) -> Self {
		Self::from_fields((
			WindowFields {
				document: Document::new(location),
			},
			EventTargetFields::default(),
			ObjectFields::default(),
		))
	}

	pub fn document(&self) -> Result<Document, WebError> {
		Ok(self.document_ref().clone())
	}

	#[must_use]
	#[expect(clippy::missing_const_for_fn, reason = "False positive")]
	pub fn document_ref(&self) -> &Document {
		&self.fields().document
	}
}
